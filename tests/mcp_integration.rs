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
async fn integration_mcp_tools_list() -> Result<(), Box<dyn std::error::Error>> {
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
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();
    let body = json!({"jsonrpc":"2.0","method":"tools/list","id":3});

    // Wait for server to become available (connection retries)
    let mut ready = false;
    for _ in 0..40 {
        match client
            .post(&url)
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await
        {
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

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&body)
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    let v: serde_json::Value = resp.json().await?;
    assert!(v.get("result").is_some(), "expected JSON-RPC result field");
    let tools = v
        .get("result")
        .unwrap()
        .get("tools")
        .and_then(|t| t.as_array());
    assert!(
        tools.is_some()
            && tools
                .unwrap()
                .iter()
                .any(|t| t.get("name").and_then(|n| n.as_str()) == Some("analyze"))
    );

    // Shutdown server
    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

#[tokio::test]
async fn integration_mcp_tools_call_missing_args() -> Result<(), Box<dyn std::error::Error>> {
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
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();
    let body = json!({
        "jsonrpc":"2.0",
        "method":"tools/call",
        "id":4,
        "params": {"name":"analyze"}
    });

    // Wait for server to become available (connection retries)
    let mut ready = false;
    for _ in 0..40 {
        match client
            .post(&url)
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await
        {
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

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&body)
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

    let v: serde_json::Value = resp.json().await?;
    assert!(v.get("error").is_some(), "expected JSON-RPC error field");
    assert!(v.get("result").is_none(), "should not have result field");
    let error = v.get("error").unwrap();
    assert_eq!(error["code"], -32602, "Expected INVALID_PARAMS error code");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Invalid parameters"));
    assert!(error["data"]["field"].as_str().unwrap() == "file_path");

    // Shutdown server
    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

#[tokio::test]
async fn integration_mcp_tools_call_batch() -> Result<(), Box<dyn std::error::Error>> {
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
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();
    // Wait for server to become available (connection retries)
    let mut ready = false;
    for _ in 0..40 {
        match client
            .post(&url)
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await
        {
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

    // Missing required `file` argument must be an INVALID_PARAMS error,
    // same contract as every other tool (never a silent fake success).
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0",
            "method":"tools/call",
            "id":4,
            "params": {"name":"batch","arguments":{}}
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);
    let v: serde_json::Value = resp.json().await?;
    assert!(
        v.get("result").is_none(),
        "missing file must not produce a result"
    );
    assert_eq!(v["error"]["code"], -32602, "expected INVALID_PARAMS");

    // Real path: preview a batch spec against a fixture file. Nothing is
    // written and the response describes the pending operations.
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("app.py");
    std::fs::write(&target, "def get_theme():\n    return 'old'\n")?;
    let spec_path = dir.path().join("batch.json");
    // Real batch file format (BatchFile/BatchOp in src/core/batch.rs):
    // edit = {file, path, content}. An earlier draft used a nonexistent
    // `replace` field and could only ever fail to parse — masked for a
    // while by the undo-test chaos (finding #12).
    let new_source = "def get_theme():\n    return 'new'\n";
    std::fs::write(
        &spec_path,
        json!({
            "description": "swap theme",
            "operations": [
                {
                    "type": "edit",
                    "file": target.to_string_lossy(),
                    "path": "0",
                    "content": new_source
                }
            ]
        })
        .to_string(),
    )?;
    let before = std::fs::read_to_string(&target)?;

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0",
            "method":"tools/call",
            "id":5,
            "params": {
                "name":"batch",
                "arguments": {
                    "file": spec_path.to_string_lossy(),
                    "preview": true
                }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let v: serde_json::Value = resp.json().await?;
    let result = v.get("result").expect("JSON-RPC result for preview");
    assert!(
        result.get("error").is_none(),
        "preview must succeed: {:?}",
        result.get("error")
    );
    let text = result["content"][0]
        .as_str()
        .unwrap_or(result["content"][0]["text"].as_str().unwrap_or(""));
    assert!(
        text.contains("preview") || text.to_lowercase().contains("nothing written"),
        "preview response must say nothing was written, got: {}",
        text
    );

    // Preview guarantee: the target file is byte-identical.
    let after = std::fs::read_to_string(&target)?;
    assert_eq!(before, after, "preview must never write to disk");

    // Shutdown server
    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

#[tokio::test]
async fn integration_mcp_tools_call_unknown_tool() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();

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

    let body = json!({
        "jsonrpc":"2.0",
        "method":"tools/call",
        "id": 6,
        "params": { "name": "unknown_tool", "arguments": {} }
    });

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&body)
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    let v: serde_json::Value = resp.json().await?;
    assert!(v.get("error").is_some(), "expected JSON-RPC error field");
    assert!(v.get("result").is_none(), "should not have result field");
    let error = v.get("error").unwrap();
    assert_eq!(
        error["code"], -32601,
        "Expected METHOD_NOT_FOUND error code"
    );
    assert!(error["message"].as_str().unwrap().contains("Unknown tool"));

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

#[tokio::test]
async fn integration_mcp_tools_call_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();

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

    let body = json!({
        "jsonrpc":"2.0",
        "method":"tools/call",
        "id": 7,
        "params": { "name": "analyze", "arguments": { "file_path": "/nonexistent/file.py" } }
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
    assert!(v.get("error").is_none(), "should not have error field");
    let result = v.get("result").unwrap();
    assert_eq!(result["isError"], true, "should be marked as error");
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("IO error"));

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

#[tokio::test]
async fn integration_mcp_tools_call_undo() -> Result<(), Box<dyn std::error::Error>> {
    // ISOLATION (GTW_MCP_ISSUE_LOG.md finding #12): undo operates on the
    // transaction log of `project_root`. An earlier version of this test
    // spawned the server with the repo as root, so every `cargo test` run
    // reverted the latest real GTW edit in the repository. The server is now
    // bound to a throwaway temp project instead.
    let temp_project = tempfile::tempdir()?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown_root(
            listener,
            token,
            temp_project.path().to_path_buf(),
            shutdown_fut,
        )
        .await
        .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();

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

    let body = json!({
        "jsonrpc":"2.0",
        "method":"tools/call",
        "id":5,
        "params": {"name":"undo","arguments":{}}
    });

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&body)
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let v: serde_json::Value = resp.json().await?;
    assert!(v.get("result").is_some());
    let result = v.get("result").unwrap();
    let content = result.get("content").and_then(|c| c.as_array()).cloned();
    assert!(content.is_some(), "undo response must carry content");
    // Deterministic outcome on an empty temp project: nothing is undoable.
    // (Asserting the message also proves the server really used the temp root.)
    let text = content
        .unwrap()
        .iter()
        .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains("Nothing to undo") || text.contains("Undo unavailable"),
        "expected empty-log undo message on temp project, got: {}",
        text
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

#[tokio::test]
async fn integration_mcp_concurrent_requests() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();

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

    // Spawn a few concurrent tasks
    let mut handles = Vec::new();
    for i in 0..3 {
        let url = url.clone();
        let token = "secret".to_string();
        let handle = tokio::spawn(async move {
            let client = Client::new();
            let body = json!({
                "jsonrpc":"2.0",
                "method":"tools/list",
                "id":10 + i
            });
            client
                .post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .json(&body)
                .send()
                .await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        let response = resp.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }
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

/// Adoption contract (ROADMAP 9.3): every tool in tools/list must be
/// self-teaching and schema-honest, so agent drift can never silently
/// regress the catalog again:
///   1. unique, non-empty name
///   2. description >= 100 chars (VAD/NÄR/RETURERAR/EXEMPEL template)
///   3. inputSchema.type == "object"
///   4. at least one property — EXCEPT honest zero-arg tools
///      (empty properties AND empty required, e.g. save_state)
///   5. every `required` entry must exist in `properties`
///   6. the ROADMAP priority tools exist with real schemas
#[tokio::test]
async fn integration_mcp_tools_adoption_contract() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();

    let mut ready = false;
    for _ in 0..40 {
        match client
            .post(&url)
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await
        {
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

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({"jsonrpc":"2.0","method":"tools/list","id":3}))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    let v: serde_json::Value = resp.json().await?;
    let tools = v
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        tools.len() >= 30,
        "expected at least 30 tools, got {}",
        tools.len()
    );

    let mut names: Vec<&str> = Vec::new();
    for tool in &tools {
        let name = tool
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or_default();
        assert!(!name.is_empty(), "tool with empty name found");
        assert!(!names.contains(&name), "duplicate tool name: {}", name);
        names.push(name);

        let desc = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or_default();
        assert!(
            desc.chars().count() >= 100,
            "tool '{}': description too short ({} chars) — must teach VAD/NÄR/RETURNERAR/EXEMPEL",
            name,
            desc.chars().count()
        );

        let schema = tool
            .get("inputSchema")
            .unwrap_or_else(|| panic!("tool '{}': missing inputSchema", name));
        assert_eq!(
            schema.get("type").and_then(|t| t.as_str()),
            Some("object"),
            "tool '{}': inputSchema.type must be 'object'",
            name
        );

        let props = schema.get("properties").and_then(|p| p.as_object());
        let required: Vec<&str> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();

        match props {
            Some(p) => {
                for req in &required {
                    assert!(
                        p.contains_key(*req),
                        "tool '{}': required '{}' not in properties (schema lies)",
                        name,
                        req
                    );
                }
            }
            None => {
                assert!(
                    required.is_empty(),
                    "tool '{}': no properties but required = {:?} (schema lies)",
                    name,
                    required
                );
            }
        }

        let props_empty = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|p| p.is_empty())
            .unwrap_or(true);
        if props_empty {
            assert!(
                required.is_empty(),
                "tool '{}': empty properties but non-empty required — a zero-arg tool must be honestly empty",
                name
            );
        }
    }

    // ROADMAP 9.3 priority tools must exist with at least one schema property.
    for priority in [
        "batch",
        "edit_node",
        "insert_node",
        "search_nodes",
        "sense",
        "semantic_edit",
    ] {
        let tool = tools
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(priority))
            .unwrap_or_else(|| panic!("priority tool '{}' missing from tools/list", priority));
        let props = tool
            .get("inputSchema")
            .and_then(|s| s.get("properties"))
            .and_then(|p| p.as_object())
            .map(|p| p.len())
            .unwrap_or(0);
        assert!(
            props >= 1,
            "priority tool '{}': expected >= 1 schema property, got {}",
            priority,
            props
        );
    }

    // Shutdown server
    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

/// ROADMAP 9.4 contract: get_semantic_report's payload carries a top-level
/// `sources` list whose entries point at the reported fixture file — the
/// provenance chain an agent uses to verify claims via read_node. Pure
/// payload assembly, so no model is needed: this test calls the exact
/// function the MCP handler calls.
#[test]
fn integration_sources_semantic_report_payload() {
    let report = gnawtreewriter::llm::SemanticReport {
        file_path: "src/core/batch.rs".into(),
        summary: "summary".into(),
        findings: vec![
            gnawtreewriter::llm::QualityFinding {
                path: "1.2".into(),
                severity: "warning".into(),
                category: "complexity".into(),
                message: "m".into(),
            },
            gnawtreewriter::llm::QualityFinding {
                path: "1.2".into(),
                severity: "info".into(),
                category: "style".into(),
                message: "m2".into(),
            },
        ],
    };
    let payload = gnawtreewriter::mcp::mcp_server::semantic_report_payload(&report);
    let sources = payload["sources"]
        .as_array()
        .expect("payload must carry sources array");
    assert_eq!(sources.len(), 1, "sources deduped by node_path");
    assert_eq!(sources[0]["file"], "src/core/batch.rs");
    assert_eq!(sources[0]["node_path"], "1.2");
    assert!(payload.get("report").is_some(), "report still present");
}

/// ROADMAP 9.4 contract: investigate's payload carries a top-level `sources`
/// list built from the evidence the answer was synthesized from.
/// (mamba-gated because InvestigateResult lives in the mamba pipeline.)
#[cfg(feature = "mamba")]
#[test]
fn integration_sources_investigate_payload() {
    let result = gnawtreewriter::llm::pipeline::InvestigateResult {
        terms: vec!["undo".into()],
        candidates: vec!["src/core/undo_redo.rs".into()],
        answer: "the answer".into(),
        sources: gnawtreewriter::llm::sources_from_evidence(&[(
            "src/core/undo_redo.rs".into(),
            "content".into(),
        )]),
    };
    let budget = gnawtreewriter::llm::TokenBudget::default();
    let payload = gnawtreewriter::mcp::mcp_server::investigate_payload(&result, &budget);
    let sources = payload["sources"]
        .as_array()
        .expect("payload must carry sources array");
    assert_eq!(sources[0]["file"], "src/core/undo_redo.rs");
    assert!(payload.get("result").is_some(), "result still present");
}

/// ROADMAP 9.5: get_skeleton must never answer with a bare header —
/// the node lines ride in content.text, and truncation is explicit.
#[tokio::test]
async fn integration_mcp_get_skeleton_never_bare_header() -> Result<(), Box<dyn std::error::Error>>
{
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();
    let mut ready = false;
    for _ in 0..40 {
        match client
            .post(&url)
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await
        {
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

    let dir = tempfile::tempdir()?;
    let target = dir.path().join("skel.rs");
    std::fs::write(&target, "fn outer() {\n    fn inner() {}\n}\nstruct S;\n")?;

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0",
            "method":"tools/call",
            "id": 2,
            "params": { "name": "get_skeleton", "arguments": { "file_path": target.to_string_lossy(), "max_depth": 2 } }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").expect("JSON-RPC result");
    assert_ne!(
        r.get("isError"),
        Some(&json!(true)),
        "skeleton parse must succeed"
    );

    // The text channel carries the actual skeleton, not just the title.
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains('\n'),
        "content.text must include node lines, got only: {:?}",
        text
    );
    assert!(
        text.contains("outer"),
        "skeleton must list named items: {:?}",
        text
    );

    // Structured fields: count + explicit truncation flag (never implied).
    assert!(r.get("skeleton").is_some(), "structured skeleton missing");
    let nodes = r["nodes"].as_u64().unwrap_or(0);
    assert!(nodes >= 1, "node count must be reported, got {}", nodes);
    assert_eq!(
        r.get("truncated"),
        Some(&json!(false)),
        "truncated flag must be explicit"
    );

    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

/// ROADMAP 9.5: `doctor` answers "is GTW alive and sane here?" in one call —
/// healthy flag + counts + per-check detail, never a bare status string.
#[tokio::test]
async fn integration_mcp_doctor_reports_health() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = oneshot::channel::<()>();
    let token = Some("secret".to_string());

    let server_handle = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = rx.await;
        };
        gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, token, shutdown_fut)
            .await
            .unwrap();
    });

    let url = format!("http://{}/", addr);
    let client = Client::new();
    let mut ready = false;
    for _ in 0..40 {
        match client
            .post(&url)
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await
        {
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

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0",
            "method":"tools/call",
            "id": 2,
            "params": { "name": "doctor", "arguments": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").expect("JSON-RPC result");
    assert_ne!(
        r.get("isError"),
        Some(&json!(true)),
        "doctor runs even when unhealthy"
    );

    // Counts + explicit health flag.
    assert!(r.get("healthy").is_some(), "healthy flag missing");
    let total = r["total"].as_u64().unwrap_or(0);
    assert!(
        total >= 20,
        "expected the full parser smoke table, got total={}",
        total
    );
    let passed = r["passed"].as_u64().unwrap_or(0);
    assert!(passed + r["failed"].as_u64().unwrap_or(0) <= total);
    let checks = r["checks"].as_array().expect("checks[] detail");
    assert_eq!(checks.len() as u64, total, "one entry per check");

    // Text channel names the tool and the verdict (guidance, not bare data).
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.starts_with("doctor:"), "got: {:?}", text);
    assert!(text.contains("checks passed"), "got: {:?}", text);

    let _ = tx.send(());
    server_handle.await?;

    Ok(())
}

/// ROADMAP 9.6: every tool_error must carry next-step guidance — no raw
/// passthrough strings. This pins the invariant mechanically (same idea
/// as the adoption contract): banned patterns must not exist in the MCP
/// server source at all.
#[test]
fn integration_error_strings_carry_guidance() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mcp/mod.rs");
    let raw = std::fs::read_to_string(&src_dir).expect("read mcp/mod.rs");
    // Strip comment lines — doc comments legitimately REFERENCE the old lies
    // ("a stub here once reported ...") as warnings; only live code counts.
    let src: String = raw
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let banned = [
        // raw passthrough of the underlying error — no context, no next step
        ("tool_error(e.to_string())", "raw e.to_string() passthrough"),
        // bare feature gate — must say how to enable it
        (
            "tool_error(\"ModernBERT feature not enabled.\".into())",
            "bare feature-gate error",
        ),
        // generic IO error without pointers
        (
            "tool_error(format!(\"IO error: {}\", e))",
            "unguided IO error",
        ),
        // silent-ok on failure paths that used to fake success
        ("\"Undo executed\"", "undo stub lie"),
        ("\"Batch executed\"", "batch stub lie"),
    ];
    for (pattern, what) in banned {
        assert!(
            !src.contains(pattern),
            "unguided error string found ({}): {} — errors must point at the next step (9.6)",
            what,
            pattern
        );
    }

    // Spot-check that guidance actually exists (the invariant has teeth).
    assert!(
        src.contains("ai setup") && src.contains("GTW_MCP_ISSUE_LOG.md"),
        "guidance must point at model setup and the issue log"
    );
}
