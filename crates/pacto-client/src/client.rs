//! JSON-RPC 2.0 client over the pacto-bot-api Unix socket.

use crate::error::PactoError;
use crate::types::{DaemonVersion, Registration};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// A registered connection to the daemon.
struct Connection {
    stream: BufStream<UnixStream>,
    next_id: u64,
    registration: Registration,
}

/// Client for sending messages into Pacto via a co-located pacto-bot-api daemon.
///
/// Connects lazily on first use, registering as a handler with the
/// `SendMessages` capability for the configured bot. If the daemon restarts,
/// the next call transparently reconnects and re-registers.
pub struct PactoClient {
    socket_path: PathBuf,
    bot_id: String,
    timeout: Duration,
    conn: Mutex<Option<Connection>>,
}

impl PactoClient {
    pub fn new(socket_path: impl Into<PathBuf>, bot_id: impl Into<String>, timeout: Duration) -> Self {
        Self {
            socket_path: socket_path.into(),
            bot_id: bot_id.into(),
            timeout,
            conn: Mutex::new(None),
        }
    }

    /// Bot identity used for outgoing messages.
    pub fn bot_id(&self) -> &str {
        &self.bot_id
    }

    /// Whether the daemon socket exists (daemon likely running).
    pub fn is_available(&self) -> bool {
        Path::new(&self.socket_path).exists()
    }

    /// Send an encrypted DM into Pacto. Returns the published event id.
    ///
    /// `recipient` is a Nostr public key (npub or hex).
    pub async fn send_dm(&self, recipient: &str, content: &str) -> Result<String, PactoError> {
        let params = json!({
            "bot_id": self.bot_id,
            "recipient": recipient,
            "content": content,
        });
        let result = self.call_with_retry("agent.send_dm", params).await?;
        result
            .as_str()
            .map(String::from)
            .ok_or_else(|| PactoError::Protocol("agent.send_dm result was not a string".into()))
    }

    /// Fetch the daemon version (doubles as a health check).
    pub async fn version(&self) -> Result<DaemonVersion, PactoError> {
        let result = self.call_with_retry("agent.version", Value::Null).await?;
        serde_json::from_value(result)
            .map_err(|e| PactoError::Protocol(format!("bad agent.version result: {e}")))
    }

    /// Issue a call, reconnecting once if the connection went stale
    /// (e.g. the daemon restarted since the last call).
    async fn call_with_retry(&self, method: &str, params: Value) -> Result<Value, PactoError> {
        let mut guard = self.conn.lock().await;

        if guard.is_none() {
            *guard = Some(self.connect_and_register().await?);
        } else if let Some(conn) = guard.as_mut() {
            match self.call(conn, method, params.clone()).await {
                Ok(result) => return Ok(result),
                // A connection drop (EOF / broken pipe, e.g. the daemon
                // restarted) means the request never completed, so it is safe
                // to reconnect and retry it once below.
                Err(PactoError::Io(e)) => {
                    warn!("Pacto daemon connection dropped ({e}), reconnecting");
                    *guard = Some(self.connect_and_register().await?);
                }
                // A timeout is ambiguous: the daemon may have already acted on
                // a non-idempotent call (agent.send_dm publishes a DM), so we
                // must NOT silently retry and risk sending twice. Tear down the
                // connection so the next call starts clean, and surface the error.
                Err(e @ PactoError::Timeout(_)) => {
                    *guard = None;
                    return Err(e);
                }
                Err(e) => return Err(e),
            }
        }

        let conn = guard.as_mut().expect("connection established above");
        let result = self.call(conn, method, params).await;
        if matches!(result, Err(PactoError::Io(_) | PactoError::Timeout(_))) {
            *guard = None;
        }
        result
    }

    /// Open the socket and complete `handler.register` for our bot.
    async fn connect_and_register(&self) -> Result<Connection, PactoError> {
        if !self.is_available() {
            return Err(PactoError::SocketNotFound(
                self.socket_path.display().to_string(),
            ));
        }

        let stream = UnixStream::connect(&self.socket_path).await?;
        let mut conn = Connection {
            stream: BufStream::new(stream),
            next_id: 1,
            registration: Registration {
                handler_id: String::new(),
                reconnect_token: String::new(),
                registered_events: Vec::new(),
            },
        };

        // Outbound-only handler: no event subscriptions, just send capability.
        let params = json!({
            "bot_ids": [self.bot_id],
            "event_types": [],
            "capabilities": ["SendMessages"],
        });
        let result = self.call(&mut conn, "handler.register", params).await?;
        conn.registration = serde_json::from_value(result)
            .map_err(|e| PactoError::Protocol(format!("bad handler.register result: {e}")))?;

        debug!(
            handler_id = %conn.registration.handler_id,
            bot_id = %self.bot_id,
            "Registered with Pacto daemon"
        );
        Ok(conn)
    }

    /// Write one request frame and read frames until our response arrives.
    async fn call(
        &self,
        conn: &mut Connection,
        method: &str,
        params: Value,
    ) -> Result<Value, PactoError> {
        let id = conn.next_id;
        conn.next_id += 1;

        let mut request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if !params.is_null() {
            request["params"] = params;
        }

        let mut line = serde_json::to_string(&request)
            .map_err(|e| PactoError::Protocol(format!("failed to serialize request: {e}")))?;
        line.push('\n');

        tokio::time::timeout(self.timeout, async {
            conn.stream.write_all(line.as_bytes()).await?;
            conn.stream.flush().await?;

            // Frames that aren't our response (daemon notifications like
            // agent.status) are skipped.
            loop {
                let mut frame = String::new();
                let n = conn.stream.read_line(&mut frame).await?;
                if n == 0 {
                    return Err(PactoError::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "daemon closed connection",
                    )));
                }
                let msg: Value = match serde_json::from_str(frame.trim()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if msg.get("id").and_then(Value::as_u64) != Some(id) {
                    let method = msg.get("method").and_then(Value::as_str);
                    debug!(?method, "Skipping daemon frame");
                    continue;
                }
                if let Some(error) = msg.get("error") {
                    return Err(PactoError::Rpc {
                        code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                        message: error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown error")
                            .to_string(),
                    });
                }
                return msg
                    .get("result")
                    .cloned()
                    .ok_or_else(|| PactoError::Protocol("response missing result".into()));
            }
        })
        .await
        .map_err(|_| PactoError::Timeout(self.timeout))?
    }
}
