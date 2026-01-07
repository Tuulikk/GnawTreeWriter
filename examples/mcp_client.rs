use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use std::time::Duration;

/// Minimal MCP example client (Rust).
///
/// Usage examples:
///   # Initialize
///   cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080 --token secret init
///
///   # List tools
///   cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080 --token secret list
///
///   # Call analyze on a file
///   cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080 --token secret analyze examples/foo.py
///
///   # Generic tools/call with JSON-encoded arguments
///   cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080 --token secret call analyze '{"file_path":"examples/foo.py"}'
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple argument parsing
    let mut args = env::args().skip(1);
    let mut url = "http://127.0.0.1:8080/".to_string();
    let mut token: Option<String> = env::var("MCP_TOKEN").ok();

    let mut command: Option<String> = None;
    let mut command_args: Vec<String> = Vec::new();

    while let Some(a) = args.next() {
        match a.as_str() {
            "--url" => {
                if let Some(u) = args.next() {
                    url = u;
                } else {
                    eprintln!("--url requires a value");
                    print_usage();
                    return Ok(());
                }
            }
            "--token" => {
                if let Some(t) = args.next() {
                    token = Some(t);
                } else {
                    eprintln!("--token requires a value");
                    print_usage();
                    return Ok(());
                }
            }
            "init" | "list" | "analyze" | "call" => {
                command = Some(a);
                // collect remaining args as command arguments
                while let Some(arg) = args.next() {
                    command_args.push(arg);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", a);
                print_usage();
                return Ok(());
            }
        }
    }

    if command.is_none() {
        println!("No command provided.\n");
        print_usage();
        return Ok(());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    match command.as_deref() {
        Some("init") => {
            wait_for_server_ready(&client, &url, token.as_deref()).await?;
            let res = rpc_initialize(&client, &url, token.as_deref()).await?;
            println!("initialize -> {}", serde_json::to_string_pretty(&res)?);
        }
        Some("list") => {
            wait_for_server_ready(&client, &url, token.as_deref()).await?;
            let res = rpc_tools_list(&client, &url, token.as_deref()).await?;
            println!("tools/list -> {}", serde_json::to_string_pretty(&res)?);
        }
        Some("analyze") => {
            if command_args.len() != 1 {
                eprintln!("analyze requires a single argument: <file_path>");
                print_usage();
                return Ok(());
            }
            let file_path = &command_args[0];
            wait_for_server_ready(&client, &url, token.as_deref()).await?;
            let res = rpc_tools_call(
                &client,
                &url,
                token.as_deref(),
                "analyze",
                json!({ "file_path": file_path }),
                10,
            )
            .await?;
            println!("analyze -> {}", serde_json::to_string_pretty(&res)?);
        }
        Some("call") => {
            if command_args.is_empty() {
                eprintln!("call requires at least 1 argument: <name> [json_args]");
                print_usage();
                return Ok(());
            }
            let name = &command_args[0];
            let args_json = if command_args.len() >= 2 {
                match serde_json::from_str::<Value>(&command_args[1]) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Invalid JSON for arguments: {}", e);
                        return Ok(());
                    }
                }
            } else {
                json!({})
            };
            wait_for_server_ready(&client, &url, token.as_deref()).await?;
            let res = rpc_tools_call(&client, &url, token.as_deref(), name, args_json, 11).await?;
            println!("tools/call -> {}", serde_json::to_string_pretty(&res)?);
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!(
        r#"Usage:
  --url <url>         MCP server URL (default: http://127.0.0.1:8080/)
  --token <token>     Bearer token (or set MCP_TOKEN env)

Commands:
  init                Call initialize
  list                Call tools/list
  analyze <file>      Call tools/call -> analyze { file_path }
  call <name> [json]  Call an arbitrary tool. json is JSON object for params.arguments

Examples:
  cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080 --token secret list
  cargo run --features mcp --example mcp_client -- --token secret analyze examples/foo.py
"#
    );
}

/// Helper: simple readiness probe via the `initialize` method.
async fn wait_for_server_ready(
    client: &Client,
    url: &str,
    token: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "id": 1
    });

    for _ in 0..40 {
        let req = client.post(url).json(&payload);
        let req = if let Some(t) = token { req.header("Authorization", format!("Bearer {}", t)) } else { req };
        match req.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    // Good enough — server responded
                    return Ok(());
                }
            }
            Err(_) => {
                // try again
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Err("server did not become ready in time".into())
}

/// Generic JSON-RPC POST helper that checks for protocol-level errors and returns `result`.
async fn rpc_post_and_extract_result(
    client: &Client,
    url: &str,
    token: Option<&str>,
    payload: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut req = client.post(url).json(&payload);
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req.send().await?;
    let body: Value = resp.json().await?;
    if let Some(err) = body.get("error") {
        return Err(format!("JSON-RPC error: {}", serde_json::to_string(err)?).into());
    }
    match body.get("result") {
        Some(r) => Ok(r.clone()),
        None => Err("No result in JSON-RPC response".into()),
    }
}

async fn rpc_initialize(
    client: &Client,
    url: &str,
    token: Option<&str>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let payload = json!({"jsonrpc":"2.0","method":"initialize","id":1});
    rpc_post_and_extract_result(client, url, token, payload).await
}

async fn rpc_tools_list(
    client: &Client,
    url: &str,
    token: Option<&str>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let payload = json!({"jsonrpc":"2.0","method":"tools/list","id":2});
    rpc_post_and_extract_result(client, url, token, payload).await
}

async fn rpc_tools_call(
    client: &Client,
    url: &str,
    token: Option<&str>,
    name: &str,
    arguments: Value,
    id: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let payload = json!({
        "jsonrpc":"2.0",
        "method":"tools/call",
        "id": id,
        "params": {
            "name": name,
            "arguments": arguments
        }
    });
    rpc_post_and_extract_result(client, url, token, payload).await
}
