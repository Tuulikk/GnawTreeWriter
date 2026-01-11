//! Rust MCP client example for GnawTreeWriter's MCP server.
//!
//! This example demonstrates how to call the MCP (Model Context Protocol) server
//! using JSON-RPC 2.0 over HTTP.
//!
//! Usage:
//!   - List tools:
//!       cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret list
//!   - Initialize:
//!       cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret init
//!   - Analyze a file:
//!       cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret analyze /path/to/file.py
//!   - Generic call:
//!       cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret call analyze '{"file_path":"examples/foo.py"}'
//!
//! Environment variables:
//!   - MCP_URL: Server URL (can override --url)
//!   - MCP_TOKEN: Bearer token (can override --token)

use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "mcp_client")]
#[command(about = "Rust MCP client example", long_about = None)]
struct Args {
    /// Server URL (default: http://127.0.0.1:8080/)
    #[arg(long)]
    url: Option<String>,

    /// Bearer token for authentication
    #[arg(long)]
    token: Option<String>,

    /// Command to execute: init, list, analyze, or call
    command: String,

    /// Arguments for command (e.g., file path for analyze)
    #[arg()]
    cmd_args: Vec<String>,
}

/// Build a JSON-RPC request
fn build_request(method: &str, id: u64, params: Option<Value>) -> JsonRpcRequest<'_> {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id,
        method,
        params,
    }
}

/// Send a JSON-RPC request to the server
async fn send_request(
    client: &Client,
    url: &str,
    token: Option<&str>,
    request: &JsonRpcRequest<'_>,
) -> Result<JsonRpcResponse> {
    let mut req_builder = client.post(url).json(request);
    if let Some(t) = token {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", t));
    }

    let response = req_builder.send().await.context("Failed to send request")?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if !status.is_success() {
        anyhow::bail!("HTTP error: {} - {}", status, body);
    }

    let rpc_response: JsonRpcResponse =
        serde_json::from_str(&body).context("Failed to parse JSON-RPC response")?;

    Ok(rpc_response)
}

/// Wait for server to be ready by calling initialize with retries
async fn wait_for_server(
    client: &Client,
    url: &str,
    token: Option<&str>,
    max_retries: u32,
    delay_ms: u64,
) -> Result<()> {
    for i in 0..max_retries {
        let req = build_request("initialize", 1, None);
        match send_request(client, url, token, &req).await {
            Ok(resp) => {
                if resp.error.is_none() {
                    return Ok(());
                }
            }
            Err(_) => {
                // Ignore errors during wait
            }
        }

        if i < max_retries - 1 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }
    anyhow::bail!("Server did not become ready after {} attempts", max_retries);
}

/// Pretty-print tool result
fn pretty_print_result(result: &Value) {
    if let Some(content) = result.get("content") {
        if let Some(arr) = content.as_array() {
            for item in arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    println!("{}", text);
                } else {
                    println!("{}", serde_json::to_string_pretty(item).unwrap());
                }
            }
        }
    }
    if let Some(structured) = result.get("structuredContent") {
        println!(
            "Structured: {}",
            serde_json::to_string_pretty(structured).unwrap()
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Use environment variables as fallback
    let url = args.url.unwrap_or_else(|| {
        std::env::var("MCP_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/".to_string())
    });
    let token_binding = args.token.or_else(|| std::env::var("MCP_TOKEN").ok());
    let token = token_binding.as_deref();

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // Wait for server to be ready
    println!("Connecting to {}...", url);
    wait_for_server(&client, &url, token, 20, 250).await?;

    let id_counter = 2u64;

    match args.command.as_str() {
        "init" => {
            println!("Calling initialize...");
            let req = build_request("initialize", 1, None);
            let resp = send_request(&client, &url, token, &req).await?;

            if let Some(err) = resp.error {
                anyhow::bail!("RPC error: {} - {}", err.code, err.message);
            }

            if let Some(result) = resp.result {
                println!("Initialize result:");
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
        }

        "list" => {
            println!("Calling tools/list...");
            let req = build_request("tools/list", 2, None);
            let resp = send_request(&client, &url, token, &req).await?;

            if let Some(err) = resp.error {
                anyhow::bail!("RPC error: {} - {}", err.code, err.message);
            }

            if let Some(result) = resp.result {
                println!("Available tools:");
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
        }

        "analyze" => {
            if args.cmd_args.len() != 1 {
                anyhow::bail!("analyze requires exactly one argument: <file_path>");
            }
            let file_path = &args.cmd_args[0];

            println!("Calling tools/call analyze for {}...", file_path);
            let params =
                serde_json::json!({ "name": "analyze", "arguments": { "file_path": file_path } });
            let req = build_request("tools/call", id_counter, Some(params));
            let resp = send_request(&client, &url, token, &req).await?;

            if let Some(err) = resp.error {
                anyhow::bail!("RPC error: {} - {}", err.code, err.message);
            }

            if let Some(result) = resp.result {
                if result
                    .get("isError")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
                {
                    anyhow::bail!(
                        "Tool error: {}",
                        serde_json::to_string_pretty(&result).unwrap()
                    );
                }
                pretty_print_result(&result);
            }
        }

        "call" => {
            if args.cmd_args.is_empty() {
                anyhow::bail!("call requires at least one argument: <tool_name> [json_args]");
            }
            let tool_name = &args.cmd_args[0];
            let arguments = if args.cmd_args.len() > 1 {
                args.cmd_args[1..].join(" ")
            } else {
                "{}".to_string()
            };

            println!("Calling tools/call {}...", tool_name);
            let args_value: Value =
                serde_json::from_str(&arguments).context("Failed to parse arguments as JSON")?;
            let params = serde_json::json!({ "name": tool_name, "arguments": args_value });
            let req = build_request("tools/call", id_counter, Some(params));
            let resp = send_request(&client, &url, token, &req).await?;

            if let Some(err) = resp.error {
                anyhow::bail!("RPC error: {} - {}", err.code, err.message);
            }

            if let Some(result) = resp.result {
                if result
                    .get("isError")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
                {
                    anyhow::bail!(
                        "Tool error: {}",
                        serde_json::to_string_pretty(&result).unwrap()
                    );
                }
                pretty_print_result(&result);
            }
        }

        _ => {
            anyhow::bail!(
                "Unknown command: {}. Available: init, list, analyze, call",
                args.command
            );
        }
    }

    Ok(())
}
