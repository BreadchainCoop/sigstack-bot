//! Full-duplex inbound agent for the pacto-bot-api daemon.
//!
//! Where [`crate::PactoClient`] is a single-flight request/response client for
//! *outbound* sends, `PactoAgent` maintains a long-lived connection that
//! registers for `dm_received` events, streams inbound DMs to a channel, and
//! replies via `handler.response`. It reconnects automatically if the daemon
//! restarts.

use crate::error::PactoError;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, info, warn};

/// A decrypted direct message delivered by the daemon (`agent.event`).
#[derive(Debug, Clone)]
pub struct InboundDm {
    /// Bot identity the message was addressed to.
    pub bot_id: String,
    /// Hex id of the enclosing gift-wrap event — used to reply via
    /// `handler.response`.
    pub event_id: String,
    /// Conversation identifier (usually the sender's npub).
    pub chat_id: Option<String>,
    /// Public key of the message author (the reply recipient).
    pub author: String,
    /// Decrypted message text.
    pub content: String,
    /// Hex id of the decrypted rumor.
    pub rumor_id: String,
    /// Unix timestamp of the rumor.
    pub timestamp: i64,
}

/// Long-lived inbound agent. Clone-cheap handle used to send replies; the
/// connection is owned by a background supervisor task spawned in [`Self::spawn`].
pub struct PactoAgent {
    write: Arc<Mutex<Option<OwnedWriteHalf>>>,
    bot_id: String,
}

impl PactoAgent {
    /// Connect (with auto-reconnect) and register for `dm_received`. Returns a
    /// reply handle plus the receiver of inbound DMs. The background task ends
    /// only when the returned receiver is dropped.
    pub fn spawn(
        socket_path: impl Into<PathBuf>,
        bot_id: impl Into<String>,
        reconnect_backoff: Duration,
    ) -> (Arc<PactoAgent>, mpsc::Receiver<InboundDm>) {
        let socket_path = socket_path.into();
        let bot_id = bot_id.into();
        let write = Arc::new(Mutex::new(None));
        let agent = Arc::new(PactoAgent {
            write: write.clone(),
            bot_id: bot_id.clone(),
        });
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(supervise(socket_path, bot_id, write, tx, reconnect_backoff));
        (agent, rx)
    }

    /// Bot identity this agent serves.
    pub fn bot_id(&self) -> &str {
        &self.bot_id
    }

    /// Reply to a delivered event. The daemon addresses the DM to the original
    /// sender and threads it to the source rumor.
    pub async fn reply(&self, event_id: &str, content: &str) -> Result<(), PactoError> {
        self.respond(json!({
            "event_id": event_id,
            "action": "reply",
            "content": content,
        }))
        .await
    }

    /// Terminate an event without replying (`ack` / `ignore` / `defer`).
    pub async fn finish(&self, event_id: &str, action: &str) -> Result<(), PactoError> {
        self.respond(json!({ "event_id": event_id, "action": action }))
            .await
    }

    async fn respond(&self, params: Value) -> Result<(), PactoError> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "handler.response",
            "params": params,
        });
        let mut guard = self.write.lock().await;
        let writer = guard
            .as_mut()
            .ok_or_else(|| PactoError::Protocol("Pacto agent not connected".into()))?;
        match write_frame(writer, &frame).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Drop the dead writer so the supervisor's reconnect can install
                // a fresh one; surface the error to the caller.
                *guard = None;
                Err(PactoError::Io(e))
            }
        }
    }
}

/// Connect, register, and pump inbound events — reconnecting forever until the
/// inbound receiver is dropped.
async fn supervise(
    socket_path: PathBuf,
    bot_id: String,
    write: Arc<Mutex<Option<OwnedWriteHalf>>>,
    tx: mpsc::Sender<InboundDm>,
    backoff: Duration,
) {
    loop {
        match connect_and_register(&socket_path, &bot_id, &write).await {
            Ok(read_half) => {
                info!(bot_id = %bot_id, "Pacto agent registered for dm_received");
                let consumer_gone = pump_events(read_half, &bot_id, &tx).await;
                *write.lock().await = None;
                if consumer_gone {
                    debug!("Pacto agent inbound receiver dropped; stopping");
                    return;
                }
                warn!("Pacto agent connection closed; reconnecting");
            }
            Err(e) => {
                if tx.is_closed() {
                    return;
                }
                warn!(error = %e, "Pacto agent connect/register failed; retrying");
            }
        }
        tokio::time::sleep(backoff).await;
    }
}

async fn connect_and_register(
    socket_path: &PathBuf,
    bot_id: &str,
    write: &Arc<Mutex<Option<OwnedWriteHalf>>>,
) -> Result<OwnedReadHalf, PactoError> {
    let stream = UnixStream::connect(socket_path).await?;
    let (read_half, mut write_half) = stream.into_split();

    let register = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "handler.register",
        "params": {
            "bot_ids": [bot_id],
            "event_types": ["dm_received"],
            "capabilities": ["ReadMessages", "SendMessages"],
        },
    });
    write_frame(&mut write_half, &register).await?;

    // Publish the write half so replies can be sent once events arrive. The
    // registration acknowledgement is consumed by the event pump below.
    *write.lock().await = Some(write_half);
    Ok(read_half)
}

/// Read frames until EOF/error. Returns `true` if the inbound receiver was
/// dropped (caller should stop), `false` if the connection just died.
async fn pump_events(read_half: OwnedReadHalf, bot_id: &str, tx: &mpsc::Sender<InboundDm>) -> bool {
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return false,
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                debug!(error = %e, "Pacto agent skipping unparseable frame");
                continue;
            }
        };

        // Registration ack / other id-correlated responses.
        if let Some(err) = msg.get("error") {
            warn!(error = %err, "Pacto daemon returned an error frame");
            continue;
        }
        match msg.get("method").and_then(Value::as_str) {
            Some("agent.event") => {
                if let Some(dm) = parse_inbound(bot_id, msg.get("params")) {
                    if tx.send(dm).await.is_err() {
                        return true;
                    }
                }
            }
            Some(other) => debug!(method = other, "Pacto agent ignoring notification"),
            None => debug!("Pacto agent registered (ack received)"),
        }
    }
}

fn parse_inbound(bot_id: &str, params: Option<&Value>) -> Option<InboundDm> {
    let p = params?;
    if p.get("type").and_then(Value::as_str) != Some("dm_received") {
        return None;
    }
    let get = |k: &str| p.get(k).and_then(Value::as_str).map(String::from);
    Some(InboundDm {
        bot_id: get("bot_id").unwrap_or_else(|| bot_id.to_string()),
        event_id: get("event_id")?,
        chat_id: get("chat_id"),
        author: get("author")?,
        content: get("content")?,
        rumor_id: get("rumor_id")?,
        timestamp: p.get("timestamp").and_then(Value::as_i64).unwrap_or(0),
    })
}

async fn write_frame(writer: &mut OwnedWriteHalf, frame: &Value) -> std::io::Result<()> {
    let mut line = serde_json::to_string(frame)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await
}
