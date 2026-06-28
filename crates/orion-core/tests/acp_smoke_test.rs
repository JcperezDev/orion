//! End-to-end smoke test for the ACP server: drive it with stdin/stdout
//! JSON-RPC messages and verify responses.

use orion_core::acp::Connection;
use orion_core::acp::types::{parse_message, ParsedMessage, RequestId};
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn acp_initialize_roundtrip() {
    // (client_writer → server_reader) and (server_writer → client_reader)
    let (client_writer_end, server_reader_end) = tokio::io::duplex(64 * 1024);
    let (server_writer_end, client_reader_end) = tokio::io::duplex(64 * 1024);
    let conn = Connection::buffered();

    // Pipe server responses to server_writer_end (so client can read from client_reader_end).
    let conn_clone = conn.clone();
    let pipe_handle = conn.clone().pipe_to(server_writer_end);
    let server = orion_core::acp::AcpServer::new(conn.clone(), Arc::new(orion_core::Config::default()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let reader_handle = tokio::spawn(async move {
        orion_core::acp::connection::run_reader(conn_clone, server_reader_end, tx).await;
    });
    let server_handle = tokio::spawn(async move {
        server.run(rx).await.unwrap();
    });

    // Client writes via client_writer_end, reads via client_reader_end.
    let mut client_writer = client_writer_end;
    let mut client_reader = BufReader::new(client_reader_end);

    client_writer
        .write_all(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "v1"}
            })
            .to_string()
            .as_bytes(),
        )
        .await
        .unwrap();
    client_writer.write_all(b"\n").await.unwrap();
    client_writer.flush().await.unwrap();
    eprintln!("[test] sent initialize");

    // Read the response line.
    let mut line = String::new();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        client_reader.read_line(&mut line).await.unwrap();
    })
    .await
    .expect("server should respond");
    eprintln!("[test] got response: {line}");

    let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], 1);
    assert_eq!(value["result"]["protocolVersion"], "v1");
    assert_eq!(value["result"]["agentInfo"]["name"], "orion");

    conn.close();
    reader_handle.abort();
    server_handle.abort();
    let _ = pipe_handle.await;
}

#[tokio::test]
async fn acp_session_new_then_prompt() {
    let (client_writer_end, server_reader_end) = tokio::io::duplex(64 * 1024);
    let (server_writer_end, client_reader_end) = tokio::io::duplex(64 * 1024);
    let conn = Connection::buffered();

    let conn_clone = conn.clone();
    let pipe_handle = conn.clone().pipe_to(server_writer_end);
    let server = orion_core::acp::AcpServer::new(conn.clone(), Arc::new(orion_core::Config::default()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let reader_handle = tokio::spawn(async move {
        orion_core::acp::connection::run_reader(conn_clone, server_reader_end, tx).await;
    });
    let server_handle = tokio::spawn(async move {
        server.run(rx).await.unwrap();
    });

    let mut client_writer = client_writer_end;
    let mut client_reader = BufReader::new(client_reader_end);

    // 1) initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"protocolVersion": "v1"}
    });
    client_writer
        .write_all(init_req.to_string().as_bytes())
        .await
        .unwrap();
    client_writer.write_all(b"\n").await.unwrap();
    client_writer.flush().await.unwrap();
    let mut line = String::new();
    client_reader.read_line(&mut line).await.unwrap();
    line.clear();

    // 2) session/new
    let cwd = std::env::current_dir().unwrap();
    let req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "session/new",
        "params": {"cwd": cwd.to_string_lossy()}
    });
    client_writer.write_all(req.to_string().as_bytes()).await.unwrap();
    client_writer.write_all(b"\n").await.unwrap();
    client_writer.flush().await.unwrap();
    client_reader.read_line(&mut line).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    let session_id = v["result"]["sessionId"].as_str().unwrap().to_string();
    assert!(!session_id.is_empty());
    line.clear();

    // 3) session/prompt (no handler → fallback stub ack)
    let req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "session/prompt",
        "params": {
            "sessionId": session_id,
            "prompt": [{"type": "text", "text": "hello"}]
        }
    });
    client_writer.write_all(req.to_string().as_bytes()).await.unwrap();
    client_writer.write_all(b"\n").await.unwrap();
    client_writer.flush().await.unwrap();
    client_reader.read_line(&mut line).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(v["result"]["stopReason"], "end_turn");

    conn.close();
    reader_handle.abort();
    server_handle.abort();
    let _ = pipe_handle.await;
}

#[tokio::test]
async fn acp_unknown_method_returns_error() {
    let (client_writer_end, server_reader_end) = tokio::io::duplex(64 * 1024);
    let (server_writer_end, client_reader_end) = tokio::io::duplex(64 * 1024);
    let conn = Connection::buffered();

    let conn_clone = conn.clone();
    let pipe_handle = conn.clone().pipe_to(server_writer_end);
    let server = orion_core::acp::AcpServer::new(conn.clone(), Arc::new(orion_core::Config::default()));
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let reader_handle = tokio::spawn(async move {
        orion_core::acp::connection::run_reader(conn_clone, server_reader_end, tx).await;
    });
    let server_handle = tokio::spawn(async move {
        server.run(rx).await.unwrap();
    });

    let mut client_writer = client_writer_end;
    let mut client_reader = BufReader::new(client_reader_end);

    client_writer
        .write_all(
            json!({"jsonrpc": "2.0", "id": 99, "method": "session/foo"})
                .to_string()
                .as_bytes(),
        )
        .await
        .unwrap();
    client_writer.write_all(b"\n").await.unwrap();
    client_writer.flush().await.unwrap();

    let mut line = String::new();
    client_reader.read_line(&mut line).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(v["error"]["code"], -32601);

    conn.close();
    reader_handle.abort();
    server_handle.abort();
    let _ = pipe_handle.await;
}

#[test]
fn parse_message_unit() {
    let v = json!({"jsonrpc": "2.0", "id": "x", "result": 42});
    match parse_message(&v).unwrap() {
        ParsedMessage::ResponseResult { id, result } => {
            assert_eq!(id, RequestId::Str("x".into()));
            assert_eq!(result, 42);
        }
        _ => panic!("expected response result"),
    }
}
