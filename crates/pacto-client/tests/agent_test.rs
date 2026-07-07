//! Tests for the inbound PactoAgent against a mock pacto-bot-api daemon.

use pacto_client::PactoAgent;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

async fn read_frame<R: AsyncBufReadExt + Unpin>(reader: &mut R) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    serde_json::from_str(line.trim()).unwrap()
}

async fn write_frame(stream: &mut UnixStream, msg: &Value) {
    let mut line = serde_json::to_string(msg).unwrap();
    line.push('\n');
    stream.write_all(line.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}

fn socket_path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("pacto.sock")
}

fn agent_event(event_id: &str, author: &str, content: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "agent.event",
        "params": {
            "bot_id": "test-bot",
            "event_id": event_id,
            "type": "dm_received",
            "chat_id": author,
            "content": content,
            "rumor_id": "rumor-1",
            "author": author,
            "timestamp": 1234,
        },
    })
}

#[tokio::test]
async fn registers_receives_dm_and_replies() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);
    let listener = UnixListener::bind(&path).unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Split so we can read the register + response while pushing an event.
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);

        // 1. Handler must register for dm_received with read+send capability.
        let register = read_frame(&mut reader).await;
        assert_eq!(register["method"], "handler.register");
        assert_eq!(register["params"]["bot_ids"], json!(["test-bot"]));
        assert_eq!(register["params"]["event_types"], json!(["dm_received"]));
        assert_eq!(
            register["params"]["capabilities"],
            json!(["ReadMessages", "SendMessages"])
        );

        // Ack the registration.
        let ack = json!({
            "jsonrpc": "2.0",
            "id": register["id"],
            "result": {
                "handler_id": "h1",
                "reconnect_token": "t1",
                "registered_events": ["dm_received"],
            },
        });
        let mut line = serde_json::to_string(&ack).unwrap();
        line.push('\n');
        write.write_all(line.as_bytes()).await.unwrap();
        write.flush().await.unwrap();

        // 2. Deliver an inbound DM.
        let ev = agent_event("evt-1", "npub1sender", "hello bot");
        let mut evline = serde_json::to_string(&ev).unwrap();
        evline.push('\n');
        write.write_all(evline.as_bytes()).await.unwrap();
        write.flush().await.unwrap();

        // 3. Expect the agent's handler.response reply, threaded to the event.
        let response = read_frame(&mut reader).await;
        assert_eq!(response["method"], "handler.response");
        assert_eq!(response["params"]["event_id"], "evt-1");
        assert_eq!(response["params"]["action"], "reply");
        assert_eq!(response["params"]["content"], "echo: hello bot");
    });

    let (agent, mut inbound) = PactoAgent::spawn(&path, "test-bot", Duration::from_millis(50));

    let dm = tokio::time::timeout(Duration::from_secs(5), inbound.recv())
        .await
        .expect("timed out waiting for inbound DM")
        .expect("inbound channel closed");
    assert_eq!(dm.author, "npub1sender");
    assert_eq!(dm.content, "hello bot");
    assert_eq!(dm.event_id, "evt-1");

    agent
        .reply(&dm.event_id, &format!("echo: {}", dm.content))
        .await
        .unwrap();

    server.await.unwrap();
}

#[tokio::test]
async fn reconnects_and_reregisters_after_daemon_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = socket_path(&dir);

    // First daemon: register, then drop the connection.
    let listener = UnixListener::bind(&path).unwrap();
    let (agent, mut inbound) = PactoAgent::spawn(&path, "test-bot", Duration::from_millis(50));

    let first = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let register = read_frame(&mut reader).await;
        assert_eq!(register["method"], "handler.register");
        let ack = json!({
            "jsonrpc": "2.0", "id": register["id"],
            "result": {"handler_id": "h1", "reconnect_token": "t1", "registered_events": ["dm_received"]},
        });
        let mut line = serde_json::to_string(&ack).unwrap();
        line.push('\n');
        write.write_all(line.as_bytes()).await.unwrap();
        write.flush().await.unwrap();
        // Drop everything → simulate daemon restart.
    });
    // The first task owns and drops the listener when it returns.
    first.await.unwrap();
    let _ = std::fs::remove_file(&path);

    // Second daemon on the same path: the agent must reconnect and re-register,
    // then deliver an event we can receive.
    let listener = UnixListener::bind(&path).unwrap();
    let second = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (read, mut write) = stream.into_split();
        let mut reader = BufReader::new(read);
        let register = read_frame(&mut reader).await;
        assert_eq!(register["method"], "handler.register");
        let ack = json!({
            "jsonrpc": "2.0", "id": register["id"],
            "result": {"handler_id": "h2", "reconnect_token": "t2", "registered_events": ["dm_received"]},
        });
        let mut line = serde_json::to_string(&ack).unwrap();
        line.push('\n');
        write.write_all(line.as_bytes()).await.unwrap();
        write.flush().await.unwrap();

        let ev = agent_event("evt-2", "npub1again", "after restart");
        let mut evline = serde_json::to_string(&ev).unwrap();
        evline.push('\n');
        write.write_all(evline.as_bytes()).await.unwrap();
        write.flush().await.unwrap();
        // Keep the connection open briefly so the reply can be observed by the
        // agent side; the test only asserts inbound delivery here.
        tokio::time::sleep(Duration::from_millis(200)).await;
    });

    let dm = tokio::time::timeout(Duration::from_secs(5), inbound.recv())
        .await
        .expect("timed out waiting for inbound DM after reconnect")
        .expect("inbound channel closed");
    assert_eq!(dm.event_id, "evt-2");
    assert_eq!(dm.content, "after restart");

    // Keep the agent alive until the inbound arrives.
    drop(agent);
    second.await.unwrap();
}
