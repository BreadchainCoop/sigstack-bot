//! Round-trip tests against a mock pacto-bot-api daemon on a Unix socket.

use pacto_client::{PactoClient, PactoError};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
use tokio::net::{UnixListener, UnixStream};

struct MockDaemon {
    listener: UnixListener,
}

impl MockDaemon {
    fn bind(path: &Path) -> Self {
        Self {
            listener: UnixListener::bind(path).unwrap(),
        }
    }

    /// Accept one connection and answer `handler.register`, then hand back
    /// the stream for scripted follow-ups.
    async fn accept_registered(&self) -> BufStream<UnixStream> {
        let (stream, _) = self.listener.accept().await.unwrap();
        let mut stream = BufStream::new(stream);

        let register = read_frame(&mut stream).await;
        assert_eq!(register["method"], "handler.register");
        assert_eq!(register["params"]["bot_ids"], json!(["test-bot"]));
        assert_eq!(register["params"]["capabilities"], json!(["SendMessages"]));

        write_frame(
            &mut stream,
            &json!({
                "jsonrpc": "2.0",
                "id": register["id"],
                "result": {
                    "handler_id": "handler-1",
                    "reconnect_token": "token-1",
                    "registered_events": [],
                },
            }),
        )
        .await;
        stream
    }
}

async fn read_frame(stream: &mut BufStream<UnixStream>) -> Value {
    let mut line = String::new();
    stream.read_line(&mut line).await.unwrap();
    serde_json::from_str(line.trim()).unwrap()
}

async fn write_frame(stream: &mut BufStream<UnixStream>, msg: &Value) {
    let mut line = serde_json::to_string(msg).unwrap();
    line.push('\n');
    stream.write_all(line.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}

fn socket_path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("pacto.sock")
}

fn client(path: &Path) -> PactoClient {
    PactoClient::new(path, "test-bot", Duration::from_secs(2))
}

#[tokio::test]
async fn send_dm_registers_then_returns_event_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);
    let daemon = MockDaemon::bind(&path);

    let server = tokio::spawn(async move {
        let mut stream = daemon.accept_registered().await;

        let send = read_frame(&mut stream).await;
        assert_eq!(send["method"], "agent.send_dm");
        assert_eq!(send["params"]["bot_id"], "test-bot");
        assert_eq!(send["params"]["recipient"], "npub1recipient");
        assert_eq!(send["params"]["content"], "hello pacto");

        write_frame(
            &mut stream,
            &json!({"jsonrpc": "2.0", "id": send["id"], "result": "eventid123"}),
        )
        .await;
    });

    let client = client(&path);
    let event_id = client.send_dm("npub1recipient", "hello pacto").await.unwrap();
    assert_eq!(event_id, "eventid123");
    server.await.unwrap();
}

#[tokio::test]
async fn daemon_notifications_are_skipped_while_waiting() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);
    let daemon = MockDaemon::bind(&path);

    let server = tokio::spawn(async move {
        let mut stream = daemon.accept_registered().await;
        let send = read_frame(&mut stream).await;

        // Unsolicited notification lands before the response.
        write_frame(
            &mut stream,
            &json!({"jsonrpc": "2.0", "method": "agent.status", "params": {"state": "ready"}}),
        )
        .await;
        write_frame(
            &mut stream,
            &json!({"jsonrpc": "2.0", "id": send["id"], "result": "eventid456"}),
        )
        .await;
    });

    let client = client(&path);
    let event_id = client.send_dm("npub1recipient", "hi").await.unwrap();
    assert_eq!(event_id, "eventid456");
    server.await.unwrap();
}

#[tokio::test]
async fn rpc_errors_are_surfaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);
    let daemon = MockDaemon::bind(&path);

    let server = tokio::spawn(async move {
        let mut stream = daemon.accept_registered().await;
        let send = read_frame(&mut stream).await;
        write_frame(
            &mut stream,
            &json!({
                "jsonrpc": "2.0",
                "id": send["id"],
                "error": {"code": -32000, "message": "unknown bot"},
            }),
        )
        .await;
    });

    let client = client(&path);
    let err = client.send_dm("npub1recipient", "hi").await.unwrap_err();
    match err {
        PactoError::Rpc { code, message } => {
            assert_eq!(code, -32000);
            assert_eq!(message, "unknown bot");
        }
        other => panic!("expected Rpc error, got {other:?}"),
    }
    server.await.unwrap();
}

#[tokio::test]
async fn reconnects_after_daemon_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);

    // First daemon: serve one send_dm, then drop the connection.
    let daemon = MockDaemon::bind(&path);
    let client = client(&path);

    let server = tokio::spawn(async move {
        let mut stream = daemon.accept_registered().await;
        let send = read_frame(&mut stream).await;
        write_frame(
            &mut stream,
            &json!({"jsonrpc": "2.0", "id": send["id"], "result": "first"}),
        )
        .await;
        // Daemon "restarts": connection and socket go away.
        drop(stream);
        drop(daemon);
    });

    assert_eq!(client.send_dm("npub1r", "one").await.unwrap(), "first");
    server.await.unwrap();
    std::fs::remove_file(&path).unwrap();

    // Second daemon on the same path: client must re-register.
    let daemon = MockDaemon::bind(&path);
    let server = tokio::spawn(async move {
        let mut stream = daemon.accept_registered().await;
        let send = read_frame(&mut stream).await;
        write_frame(
            &mut stream,
            &json!({"jsonrpc": "2.0", "id": send["id"], "result": "second"}),
        )
        .await;
    });

    assert_eq!(client.send_dm("npub1r", "two").await.unwrap(), "second");
    server.await.unwrap();
}

#[tokio::test]
async fn missing_socket_is_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);

    let client = client(&path);
    let err = client.send_dm("npub1r", "hi").await.unwrap_err();
    assert!(matches!(err, PactoError::SocketNotFound(_)));
}

#[tokio::test]
async fn version_parses_daemon_info() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);
    let daemon = MockDaemon::bind(&path);

    let server = tokio::spawn(async move {
        let mut stream = daemon.accept_registered().await;
        let req = read_frame(&mut stream).await;
        assert_eq!(req["method"], "agent.version");
        write_frame(
            &mut stream,
            &json!({
                "jsonrpc": "2.0",
                "id": req["id"],
                "result": {"version": "0.6.0", "commit": "abcd1234"},
            }),
        )
        .await;
    });

    let client = client(&path);
    let version = client.version().await.unwrap();
    assert_eq!(version.version, "0.6.0");
    assert_eq!(version.commit.as_deref(), Some("abcd1234"));
    server.await.unwrap();
}
