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

/// Motor2 plan #25: stable error codes must stay in sync with their
/// registry — every code emitted via tool_error_code appears in
/// docs/ERROR_CODES.md, and vice versa (no stale doc lines, no
/// undocumented codes).
#[test]
fn integration_error_codes_documented() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(manifest.join("src/mcp/mod.rs")).expect("mcp/mod.rs");
    let docs =
        std::fs::read_to_string(manifest.join("docs/ERROR_CODES.md")).expect("docs/ERROR_CODES.md");

    // Codes emitted: every "E_..." string literal in the server source
    // (call sites may span lines, so scan the whole text, not per line).
    let mut codes_in_code: Vec<String> = Vec::new();
    let mut rest = src.as_str();
    while let Some(start) = rest.find("\"E_") {
        let tail = &rest[start + 1..];
        if let Some(end) = tail.find("\"") {
            let code = tail[..end].to_string();
            if !codes_in_code.contains(&code) {
                codes_in_code.push(code);
            }
            rest = &tail[end + 1..];
        } else {
            break;
        }
    }
    assert!(
        !codes_in_code.is_empty(),
        "expected at least one coded tool_error"
    );
    for code in &codes_in_code {
        assert!(
            docs.contains(&format!("`{}`", code)),
            "code {} emitted in mcp/mod.rs but missing from docs/ERROR_CODES.md",
            code
        );
    }

    // Docs must not rot either: every E_ code in the registry is emitted
    // somewhere (registry lines look like: | `E_CODE` | meaning | ...).
    for line in docs.lines() {
        if let Some(start) = line.find("`E_") {
            let tail = &line[start + 1..];
            if let Some(end) = tail.find("`") {
                let code = tail[..end].to_string();
                assert!(
                    src.contains(&format!("\"{}\"", code)),
                    "docs list {} but no tool_error_code emits it",
                    code
                );
            }
        }
    }
}

/// Motor2 bug 2 (issue log 2026-10-05): index_project builds the VECTOR
/// index satellite sense searches (index_entities = knowledge graph).
/// Status must be safe to poll — never kicks a run — and unknown actions
/// fail loudly with the valid ones.
#[tokio::test]
async fn integration_mcp_index_project_status() -> Result<(), Box<dyn std::error::Error>> {
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

    // status on a fresh server: no run, no side effects, actionable text.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0",
            "method":"tools/call",
            "id": 2,
            "params": { "name": "index_project", "arguments": { "action": "status" } }
        }))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").expect("result");
    assert_ne!(r.get("isError"), Some(&json!(true)));
    assert_eq!(
        r["running"],
        json!(false),
        "fresh status must not be running"
    );
    assert!(r["last_run"].is_null(), "no prior run on a fresh server");
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("No indexing run yet"), "got: {:?}", text);
    assert!(
        text.contains("index_project") || text.contains("start"),
        "must say how to start"
    );

    // unknown action: loud, with the valid set.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0",
            "method":"tools/call",
            "id": 3,
            "params": { "name": "index_project", "arguments": { "action": "banana" } }
        }))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").expect("result");
    assert_eq!(
        r.get("isError"),
        Some(&json!(true)),
        "unknown action must error"
    );
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("start") && text.contains("status"),
        "got: {:?}",
        text
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

/// Anti-drift for the agent-facing catalog: every registered tool must
/// appear (backticked) in GTW_INSTRUCTIONS.md — the file that has
/// drifted twice (17 -> 30 tools, then 30 -> 34 when index_project/
/// history/stats/doctor landed without updating it).
#[test]
fn integration_mcp_instructions_listed() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(manifest.join("src/mcp/mod.rs")).expect("mcp/mod.rs");
    let doc =
        std::fs::read_to_string(manifest.join("GTW_INSTRUCTIONS.md")).expect("GTW_INSTRUCTIONS");

    let mut names: Vec<String> = Vec::new();
    let mut rest = src.as_str();
    while let Some(start) = rest.find("\"name\": \"") {
        let tail = &rest[start + "\"name\": \"".len()..];
        if let Some(end) = tail.find("\"") {
            let name = tail[..end].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
            rest = &tail[end + 1..];
        } else {
            break;
        }
    }
    assert!(
        names.len() >= 30,
        "expected the full registry, found only {}",
        names.len()
    );
    for name in &names {
        assert!(
            doc.contains(&format!("`{}`", name)),
            "tool {} is registered but missing from GTW_INSTRUCTIONS.md — update the table with it",
            name
        );
    }
}

/// Motor2 plan #23: duplex metrics persist at
/// <project>/.gnawtreewriter_metrics.json with monotonic counters.
#[test]
fn integration_duplex_metrics_persist() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    gnawtreewriter::mcp::mcp_server::bump_duplex_metric(root, "proposed");
    gnawtreewriter::mcp::mcp_server::bump_duplex_metric(root, "proposed");
    gnawtreewriter::mcp::mcp_server::bump_duplex_metric(root, "applied");

    let raw = std::fs::read_to_string(root.join(".gnawtreewriter_metrics.json"))
        .expect("metrics file written");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    assert_eq!(v["duplex"]["proposed"], json!(2));
    assert_eq!(v["duplex"]["applied"], json!(1));
    assert!(
        v["duplex"]["validated"].is_null(),
        "untouched keys stay absent (consumers read null/0)"
    );

    // A second round keeps accumulating (not overwritten).
    gnawtreewriter::mcp::mcp_server::bump_duplex_metric(root, "applied");
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".gnawtreewriter_metrics.json")).expect("read back"),
    )
    .expect("valid json");
    assert_eq!(v["duplex"]["applied"], json!(2));

    // Corrupt pre-existing file must not panic — starts fresh.
    let dir2 = tempfile::tempdir().expect("tempdir2");
    std::fs::write(dir2.path().join(".gnawtreewriter_metrics.json"), "not json").unwrap();
    gnawtreewriter::mcp::mcp_server::bump_duplex_metric(dir2.path(), "rejected");
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir2.path().join(".gnawtreewriter_metrics.json"))
            .expect("read back"),
    )
    .expect("valid json");
    assert_eq!(v["duplex"]["rejected"], json!(1));
}

/// Motor2 brief (partial-grace): a localized syntax error must not void
/// the file for READ paths — skeleton/analyze return the partial tree
/// plus an explicit syntax_warning — while EDITORS stay strict (same
/// file, E_EDIT_REJECTED, no bytes written).
#[tokio::test]
async fn integration_mcp_partial_parse_read_paths() -> Result<(), Box<dyn std::error::Error>> {
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
    let target = dir.path().join("partial.rs");
    let broken = "fn healthy() -> i32 {\n    42\n}\nfn broken( {\n";
    std::fs::write(&target, broken)?;

    let call = |name: &str, args: serde_json::Value| {
        json!({"jsonrpc":"2.0", "method":"tools/call", "id": 7,
               "params": {"name": name, "arguments": args}})
    };

    // 1. get_skeleton: partial content + explicit warning, NOT an error.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&call(
            "get_skeleton",
            json!({"file_path": target.to_string_lossy(), "max_depth": 3}),
        ))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").unwrap();
    assert_ne!(
        r.get("isError"),
        Some(&json!(true)),
        "read must not fail whole-file"
    );
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("PARTIAL PARSE"),
        "warning in text, got: {:?}",
        text
    );
    assert!(
        text.contains("healthy"),
        "partial tree keeps what parsed: {:?}",
        text
    );
    assert!(r["syntax_warning"]["line"].as_u64().unwrap_or(0) >= 1);

    // 2. analyze: same guarantee on the data channel.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&call(
            "analyze",
            json!({"file_path": target.to_string_lossy()}),
        ))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").unwrap();
    assert_ne!(r.get("isError"), Some(&json!(true)));
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("PARTIAL PARSE"), "got: {:?}", text);
    assert!(r["data"]["syntax_warning"]["line"].as_u64().unwrap_or(0) >= 1);

    // 3. edit on the same file: STRICT — rejected with a stable code,
    //    and the file stays byte-identical (no partial garbage writes).
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&call(
            "edit_node",
            json!({
                "file_path": target.to_string_lossy(),
                "node_path": "0",
                "content": "fn healthy() -> i32 { 7 }\nfn broken( {\n"
            }),
        ))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").unwrap();
    assert_eq!(
        r.get("isError"),
        Some(&json!(true)),
        "editors must stay strict"
    );
    assert_eq!(
        r.get("code").and_then(|c| c.as_str()),
        Some("E_STRICT_PARSE"),
        "strict refusal of a broken file is E_STRICT_PARSE (not a missing file), got: {:?}",
        r
    );
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("partial") || text.contains("analyze"),
        "refusal must point at the partial read paths, got: {:?}",
        text
    );
    assert_eq!(
        std::fs::read_to_string(&target)?,
        broken,
        "no bytes written on refusal"
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

/// Phase 10 surfaces in one session: guide (coaching + tool pinning),
/// validate (syntax gate), diff (independent verification), history/stats
/// (introspection) and undo preview (see-before-revert, zero mutation).
#[tokio::test]
async fn integration_mcp_phase10_surfaces() -> Result<(), Box<dyn std::error::Error>> {
    // Hermetic: the server is rooted in a throwaway project (git marker +
    // fixture files) so history/undo preview NEVER depend on the checked-out
    // repo's transaction log — CI runs on a fresh clone where the log is
    // gitignored and contains nothing undoable (the exact CI failure this
    // test caused before being made hermetic).
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join(".git"))?;
    let root_buf = dir.path().to_path_buf();

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
            root_buf,
            shutdown_fut,
        )
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

    let call = |name: &str, args: serde_json::Value| {
        json!({"jsonrpc":"2.0", "method":"tools/call", "id": 9,
               "params": {"name": name, "arguments": args}})
    };
    let post = |body: serde_json::Value| {
        let url = url.clone();
        let client = client.clone();
        async move {
            let resp = client
                .post(&url)
                .header("Authorization", "Bearer secret")
                .json(&body)
                .send()
                .await
                .expect("http");
            let v: serde_json::Value = resp.json().await.expect("json");
            v.get("result").expect("result").clone()
        }
    };

    // --- guide: full catalog, every pinned tool must be registered.
    let resp_list = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({"jsonrpc":"2.0","method":"tools/list","id":1}))
        .send()
        .await?;
    let vl: serde_json::Value = resp_list.json().await?;
    let registered: Vec<String> = vl["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect();

    let r = post(call("guide", json!({}))).await;
    assert_ne!(r.get("isError"), Some(&json!(true)));
    let situations = r["situations"].as_array().expect("catalog");
    assert!(
        situations.len() >= 20,
        "full catalog, got {}",
        situations.len()
    );
    for s in situations {
        let tool = s["tool"].as_str().expect("tool field");
        assert!(
            registered.iter().any(|t| t == tool),
            "guide pins unknown tool {:?} — not in tools/list",
            tool
        );
        assert!(s["example"].as_str().unwrap_or("").contains('{'));
    }

    // guide filter (matches on when/tool/example — the doc example query).
    let r = post(call("guide", json!({"situation": "undo a bad edit"}))).await;
    assert_ne!(r.get("isError"), Some(&json!(true)), "filter must hit");
    let rows = r["situations"].as_array().unwrap();
    assert!(rows.iter().any(|s| s["tool"] == "undo"));

    // guide no-match: loud, not empty.
    let r = post(call("guide", json!({"situation": "zzz impossible"}))).await;
    assert_eq!(r.get("isError"), Some(&json!(true)));

    // --- validate: clean file green, broken file fails with the stable code.
    let clean = dir.path().join("clean.rs");
    std::fs::write(&clean, "fn ok() -> i32 { 1 }")?;
    let r = post(call(
        "validate",
        json!({"file_path": clean.to_string_lossy()}),
    ))
    .await;
    assert_ne!(r.get("isError"), Some(&json!(true)));
    assert_eq!(r["valid"], json!(true));
    assert!(r["nodes"].as_u64().unwrap_or(0) >= 1);

    let broken = dir.path().join("broken.rs");
    std::fs::write(&broken, "fn broken( {")?;
    let r = post(call(
        "validate",
        json!({"file_path": broken.to_string_lossy()}),
    ))
    .await;
    assert_eq!(r.get("isError"), Some(&json!(true)));
    assert_eq!(
        r.get("code").and_then(|c| c.as_str()),
        Some("E_STRICT_PARSE"),
        "validate failure carries the stable code: {:?}",
        r
    );
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("partial"),
        "must point at read paths: {text:?}"
    );

    // --- diff: file mode + missing-args guidance.
    let before = dir.path().join("before.txt");
    let after = dir.path().join("after.txt");
    std::fs::write(&before, "line one\n")?;
    std::fs::write(&after, "line one\ntwo\n")?;
    let r = post(call(
        "diff",
        json!({"old_file": before.to_string_lossy(), "new_file": after.to_string_lossy()}),
    ))
    .await;
    assert_ne!(r.get("isError"), Some(&json!(true)));
    assert_eq!(r["mode"], json!("files"));
    let dtext = r["diff"].as_str().unwrap_or("");
    assert!(!dtext.is_empty(), "diff text must not be empty");

    let r = post(call("diff", json!({}))).await;
    assert_eq!(r.get("isError"), Some(&json!(true)));
    let text = r["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        text.contains("old_file"),
        "must teach the two modes: {text:?}"
    );

    // --- stats: real numbers from the project.
    let r = post(call("stats", json!({}))).await;
    assert_ne!(r.get("isError"), Some(&json!(true)));
    assert!(
        r["total_files"].as_u64().unwrap_or(0) >= 1,
        "stats must count the repo: {:?}",
        r
    );

    // --- history + undo preview (hermetic): empty project first, then a
    // real edit creates ONE undoable op; preview lists it and mutates
    // nothing (history count proves it).
    let r = post(call("history", json!({"limit": 500}))).await;
    assert_eq!(
        r["count"],
        json!(1),
        "fresh project logs exactly the SessionStart: {r:?}"
    );
    assert_eq!(
        r["transactions"][0]["operation"],
        json!("SessionStart"),
        "load() opens the session on first use"
    );

    let r = post(call("undo", json!({"steps": 1, "preview": true}))).await;
    assert_eq!(
        r.get("isError"),
        Some(&json!(true)),
        "preview on empty log must say Nothing to undo: {:?}",
        r["content"]
    );

    // One reversible op in THIS project.
    let target = dir.path().join("f.rs");
    std::fs::write(&target, "fn a() {}\n")?;
    let r = post(call(
        "edit_node",
        json!({"file_path": target.to_string_lossy(), "node_path": "0", "content": "fn a() { /*x*/ }\n"}),
    ))
    .await;
    assert_ne!(
        r.get("isError"),
        Some(&json!(true)),
        "edit must work: {r:?}"
    );
    assert!(
        !r["transaction_id"].as_str().unwrap_or("").is_empty(),
        "write receipt in passing flow too"
    );
    let count_after = post(call("history", json!({"limit": 500}))).await;
    assert_eq!(count_after["count"], json!(2), "SessionStart + the edit");

    let r = post(call("undo", json!({"steps": 1, "preview": true}))).await;
    assert_ne!(
        r.get("isError"),
        Some(&json!(true)),
        "preview must list the op: {:?}",
        r["content"]
    );
    assert_eq!(r["preview"], json!(true));
    let ops = r["operations"].as_array().expect("preview operations");
    assert_eq!(ops.len(), 1, "exactly our op");
    assert!(ops[0].get("id").is_some() && ops[0].get("file").is_some());

    let after = post(call("history", json!({"limit": 500}))).await;
    assert_eq!(
        after["count"].as_u64().unwrap_or(0),
        2,
        "preview must not write or revert anything"
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

/// Phase 10 receipts + idempotency: writes carry transaction_id; a
/// retried edit with identical content succeeds with already_applied
/// (no bytes rewritten — the host-timeout retry case).
#[tokio::test]
async fn integration_mcp_write_receipts_idempotent() -> Result<(), Box<dyn std::error::Error>> {
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

    // Isolated project: .git marker makes find_project_root stop here, so
    // transactions never touch the real repo log.
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join(".git"))?;
    let target = dir.path().join("f.rs");
    std::fs::write(&target, "fn a() {}\n")?;

    let edit = |content: &str| {
        json!({"jsonrpc":"2.0", "method":"tools/call", "id": 5,
        "params": {"name": "edit_node", "arguments": {
            "file_path": target.to_string_lossy(),
            "node_path": "0",
            "content": content
        }}})
    };

    // First write: receipt present.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&edit("fn a() { /* v2 */ }\n"))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").unwrap();
    assert_ne!(r.get("isError"), Some(&json!(true)), "{:?}", r["content"]);
    let txn = r["transaction_id"].as_str().unwrap_or("");
    assert!(
        !txn.is_empty(),
        "write must carry a transaction_id receipt: {r:?}"
    );
    assert!(r["diff"].as_str().unwrap_or("").contains("v2"));

    // Retry with identical content: idempotent success, no rewrite.
    let bytes_before = std::fs::read_to_string(&target)?;
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&edit("fn a() { /* v2 */ }\n"))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").unwrap();
    assert_ne!(
        r.get("isError"),
        Some(&json!(true)),
        "retry must not fail: {r:?}"
    );
    assert_eq!(
        r["already_applied"],
        json!(true),
        "expected idempotent hit: {r:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&target)?,
        bytes_before,
        "no bytes rewritten"
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

/// Fas 5.1/5.3 contract: a rejected edit must carry a structured
/// `edit_verdict` (level, findings, suggestions) in the MCP response.
#[tokio::test]
async fn integration_mcp_edit_rejection_carries_edit_verdict(
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Isolated project (hermetic — never operate on repo root state).
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join(".git"))?;
    let target = dir.path().join("f.rs");
    let guarded = "fn get(items: &[u8], i: usize) -> u8 {\n    if i < items.len() {\n        items[i]\n    } else {\n        0\n    }\n}\n";
    std::fs::write(&target, guarded)?;

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&json!({
            "jsonrpc":"2.0", "method":"tools/call", "id": 9,
            "params": {"name": "edit_node", "arguments": {
                "file_path": target.to_string_lossy(),
                "node_path": "@fn:get",
                "content": "fn get(items: &[u8], i: usize) -> u8 {\n    items[i]\n}\n"
            }}
        }))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v
        .get("result")
        .expect("tool-level errors are results, not rpc errors");
    assert_eq!(r.get("isError"), Some(&json!(true)), "{:?}", r["content"]);
    assert_eq!(r["code"], json!("E_EDIT_REJECTED"));

    let verdict = r
        .get("edit_verdict")
        .expect("rejected edit must carry edit_verdict");
    assert_eq!(verdict["level"], json!("critical"));
    let findings = verdict["findings"].as_array().expect("findings array");
    assert!(
        !findings.is_empty(),
        "findings must be present: {verdict:?}"
    );
    assert!(
        findings.iter().any(|f| f["message"]
            .as_str()
            .is_some_and(|m| m.contains("guard") || m.contains("condition"))),
        "findings must name the dropped guard: {verdict:?}"
    );
    let suggestions = verdict["suggestions"]
        .as_array()
        .expect("suggestions array");
    assert!(
        !suggestions.is_empty(),
        "suggestions must be present: {verdict:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&target)?,
        guarded,
        "rejected edit must not touch the file"
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}

/// Fas 4 contract: a signature change on an indexed project carries the
/// impacted call sites in the MCP success response (`impact`); a
/// body-only edit omits the field entirely.
#[tokio::test]
async fn integration_mcp_edit_reports_impact_on_signature_change(
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Hermetic indexed project: definition + caller + knowledge graph.
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join(".git"))?;
    let def_path = dir.path().join("lib.rs");
    let call_path = dir.path().join("caller.rs");
    std::fs::write(&def_path, "fn target(alpha: u32) -> u32 {\n    alpha\n}\n")?;
    std::fs::write(&call_path, "fn caller() -> u32 {\n    target(1)\n}\n")?;
    let mut indexer = gnawtreewriter::llm::RelationalIndexer::new(dir.path());
    indexer.index_directory(dir.path())?;

    let call_edit = |id: i32, content: &str| {
        json!({
            "jsonrpc":"2.0", "method":"tools/call", "id": id,
            "params": {"name": "edit_node", "arguments": {
                "file_path": def_path.to_string_lossy(),
                "node_path": "@fn:target",
                "content": content
            }}
        })
    };

    // Signature widening → impact field with the caller site.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&call_edit(
            11,
            "fn target(alpha: u32, beta: u32) -> u32 {\n    alpha + beta\n}\n",
        ))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").expect("success expected");
    assert_ne!(r.get("isError"), Some(&json!(true)), "{:?}", r["content"]);
    let impact = r.get("impact").expect("signature change must carry impact");
    assert_eq!(impact["callers"], 1);
    assert!(
        impact["sites"][0]
            .as_str()
            .unwrap_or("")
            .contains("caller.rs"),
        "site must name the caller file: {impact:?}"
    );

    // Control: body-only change → impact omitted, not null.
    let resp = client
        .post(&url)
        .header("Authorization", "Bearer secret")
        .json(&call_edit(
            12,
            "fn target(alpha: u32, beta: u32) -> u32 {\n    alpha * beta\n}\n",
        ))
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    let r = v.get("result").expect("success expected");
    assert_ne!(r.get("isError"), Some(&json!(true)), "{:?}", r["content"]);
    assert!(
        r.get("impact").is_none(),
        "body-only change must omit impact: {r:?}"
    );

    let _ = tx.send(());
    server_handle.await?;
    Ok(())
}
