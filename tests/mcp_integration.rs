#![cfg(feature = "mcp")]

use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::sleep;

/// Integration test for MCP server:
/// - binds an ephemeral listener
/// - starts server with a secret token
/// - checks that unauthenticated requests get 401
/// - checks that authenticated requests get 200 and a JSON-RPC result
#[tokio::test]
async fn integration_mcp_auth() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // oneshot channel to signal server shutdown
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    // Spawn the server; it will run until we send on `tx`
    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        // Note: serve_with_shutdown is implemented in the mcp module
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();
    let body = json!({"jsonrpc":"2.0","method":"initialize","id":1});

    // Wait for server to become available (connection retries)
    let mut ready = false;
    for _ in 0..40 {
        match client.post(&url).json(&body).send().await {
            Ok(_) => {
                ready = true;
                break;
            }
            Err(e) => {
                if e.is_connect() {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                } else {
                    // other error (parsing, timeout, etc.) — break and let test fail later
                    break;
                }
            }
        }
    }
    assert!(ready, "server did not become ready in time");

    // 1) unauthorized: no Authorization header => 401
    let res = client.post(&url).json(&body).send().await?;
    assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

    // 2) authorized: Authorization: Bearer secret => 200 + JSON-RPC result
    let res2 = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&body)
        .send()
        .await?;
    assert_eq!(res2.status(), reqwest::StatusCode::OK);

    let v: serde_json::Value = res2.json().await?;
    assert!(v.get("result").is_some(), "expected JSON-RPC result field");

    // Shutdown server
    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

#[tokio::test]
async fn integration_mcp_tools_call_analyze() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // oneshot channel to signal server shutdown
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    // Spawn the server; it will run until we send on `tx`
    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        // Note: serve_with_shutdown is implemented in the mcp module
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();

    // create temp file to analyze
    let mut tmp = tempfile::NamedTempFile::new()?;
    use std::io::Write;
    write!(tmp, "def foo():\n    return 42\n")?;
    let path = tmp.path().to_str().unwrap().to_string();

    // Wait for server to become available (connection retries)
    let body_init = json!({"jsonrpc":"2.0","method":"initialize","id":1});
    let mut ready = false;
    for _ in 0..40 {
        match client.post(&url).json(&body_init).send().await {
            Ok(_) => {
                ready = true;
                break;
            }
            Err(e) => {
                if e.is_connect() {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                } else {
                    break;
                }
            }
        }
    }
    assert!(ready, "server did not become ready in time");

    // Call tools/call analyze
    let body = json!({
        "jsonrpc":"2.0",
        "method":"tools/call",
        "id": 2,
        "params": { "name": "analyze", "arguments": { "file_path": path } }
    });

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&body)
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let v: serde_json::Value = resp.json().await?;
    assert!(v.get("result").is_some(), "expected JSON-RPC result field");
    let res_obj = v.get("result").unwrap();
    assert!(res_obj.get("structuredContent").is_some() || res_obj.get("content").is_some());

    // Shutdown server
    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}
