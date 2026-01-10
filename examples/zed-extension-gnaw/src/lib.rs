//! Minimal Zed extension skeleton that exposes GnawTreeWriter as an MCP context server.
//!
//! This is a lightweight, ready-to-edit template intended for local/dev usage.
//! It demonstrates how to implement `context_server_command` (see Zed docs:
//! https://zed.dev/docs/extensions/mcp-extensions).
//!
//! Notes:
//! - You will likely need to pin the `zed` crate version in `Cargo.toml` and adapt imports
//!   to the exact API available in your environment.
//! - This skeleton prefers an installed `gnawtreewriter` binary when available and falls
//!   back to running the local `./scripts/mcp-serve.sh` script.
//!
//! Integration checklist (manual):
//! - Add/adjust the extension manifest `extension.toml` (already present in this example).
//! - Build the extension (with `cargo build --release`) and follow Zed docs to install as a dev extension.
//! - Configure address and token via project settings or environment variables as needed.

use zed_extension_api as zed;

#[cfg(not(target_arch = "wasm32"))]
fn has_gnaw_binary() -> bool {
    // On host builds we can probe PATH for the `gnawtreewriter` binary.
    // This call is not compiled for wasm targets.
    which::which("gnawtreewriter").is_ok()
}

#[cfg(target_arch = "wasm32")]
fn has_gnaw_binary() -> bool {
    // wasm can't check PATH; assume binary is not available.
    false
}

/// Extension type. Keep state here if needed.
pub struct GnawExtension {}

impl GnawExtension {
    /// Create new instance of the extension.
    pub fn new() -> Self {
        Self {}
    }
}

/// Helper: Returns (command, args, env) to start the local MCP server.
///
/// Preference order:
/// 1. If `gnawtreewriter` binary is on PATH -> use that with `mcp serve`.
/// 2. Otherwise call the local script `./scripts/mcp-serve.sh`.
///
/// Users should adapt `ADDR`/`TOKEN` or make them configurable via project settings/env.
fn preferred_server_invocation() -> (String, Vec<String>, Vec<(String, String)>) {
    // Default settings (simple defaults for local dev)
    const ADDR: &str = "127.0.0.1:8080";
    const TOKEN: &str = "secret";

    // If the gnawtreewriter binary is available on PATH, prefer it (checked platform‑safely).
    // `has_gnaw_binary()` is a cfg‑gated helper that avoids using `which` on wasm targets.
    if has_gnaw_binary() {
        let mut args = Vec::new();
        args.push("mcp".into());
        args.push("serve".into());
        args.push("--addr".into());
        args.push(ADDR.into());
        args.push("--token".into());
        args.push(TOKEN.into());
        (String::from("gnawtreewriter"), args, Vec::new())
    } else {
        // Fallback to direct script invocation (avoid passing everything through 'sh -c')
        // This avoids quoting/concatenation issues caused by `sh -c "..."` and ensures
        // each flag is passed as a separate argv entry.
        let cmd = String::from("./scripts/mcp-serve.sh");
        let args = vec!["--addr".into(), ADDR.into(), "--token".into(), TOKEN.into()];
        let env = vec![("MCP_TOKEN".to_string(), TOKEN.to_string())];
        (cmd, args, env)
    }
}

#[cfg(feature = "zed")]
impl zed::Extension for GnawExtension {
    /// Required associated constructor for the Extension trait.
    fn new() -> Self {
        GnawExtension {}
    }

    /// Return the command Zed should run to start the context server.
    ///
    /// The trait expects `Result<Command, String>` as the return type (string errors).
    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> std::result::Result<zed::process::Command, String> {
        let (command, args, env) = preferred_server_invocation();

        // If we will use the direct script invocation (`./scripts/mcp-serve.sh`) perform a few
        // protective checks and return helpful errors when common issues are detected.
        // This makes failures actionable (e.g. if the dev extension was installed from the wrong dir
        // or passed malformed args such as '--addr127.0.0.1:8080').
        if command.ends_with("mcp-serve.sh") || command == "./scripts/mcp-serve.sh" {
            let script_path = std::path::Path::new("./scripts/mcp-serve.sh");
            // Include the current working directory in error messages to make debugging easier.
            let cwd = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<unknown>".into());
            if !script_path.exists() || !script_path.is_file() {
                return Err(format!(
                    "Cannot find './scripts/mcp-serve.sh' in the current working directory: {}. \
Ensure you installed the dev extension from the repository root or copy the repository's 'scripts/' directory into the extension folder. \
Alternatively, install the 'gnawtreewriter' binary on PATH so the extension can use it directly.",
                    cwd
                ));
            }
            // Ensure the script is executable. On host builds try to make it executable using Zed helper.
            match std::fs::metadata(script_path) {
                Ok(meta) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if (meta.permissions().mode() & 0o111) == 0 {
                            // Attempt to make the file executable via Zed helper to avoid permission issues.
                            // If this fails, return a helpful error including the CWD.
                            if let Err(e) = zed::make_file_executable(script_path) {
                                return Err(format!(
                                    "'./scripts/mcp-serve.sh' exists but is not executable and an attempt to make it executable failed: {:?}. CWD: {}",
                                    e, cwd
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to stat './scripts/mcp-serve.sh': {}. CWD: {}",
                        e, cwd
                    ));
                }
            }

            // Check for malformed concatenated flags (e.g. --addr127 or --tokensecret).
            for a in args.iter() {
                if a.starts_with("--addr") && !a.starts_with("--addr=") && a != "--addr" {
                    return Err(
                        "Malformed argument detected: '--addr' appears concatenated to its value (e.g. '--addr127.0.0.1'). \
Use '--addr 127.0.0.1:8080' or '--addr=127.0.0.1:8080' instead.".to_string(),
                    );
                }
                if a.starts_with("--token") && !a.starts_with("--token=") && a != "--token" {
                    return Err(
                        "Malformed argument detected: '--token' appears concatenated to its value (e.g. '--tokensecret'). \
Use '--token secret' or '--token=secret' instead.".to_string(),
                    );
                }
            }
        }

        Ok(zed::process::Command { command, args, env })
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> std::result::Result<Option<zed::ContextServerConfiguration>, String> {
        // Provide installation instructions and a simple settings schema/defaults so Zed can
        // show the toggle and allow the user to configure the server settings in the UI.
        let installation_instructions = "Start the MCP server from the Agent panel or run `./scripts/mcp-serve.sh --addr <host:port> --token <token>`".to_string();

        // Simple JSON schema (string form) for settings validation in Zed UI.
        let settings_schema = r#"{
  "type": "object",
  "properties": {
    "addr": { "type": "string", "title": "Bind address", "default": "127.0.0.1:8080" },
    "token": { "type": "string", "title": "Bearer token", "default": "secret" }
  }
}"#
        .to_string();

        // Default settings (string form) that will populate the UI fields initially.
        let default_settings = r#"{
  "addr": "127.0.0.1:8080",
  "token": "secret"
}"#
        .to_string();

        let cfg = zed::ContextServerConfiguration {
            installation_instructions,
            settings_schema,
            default_settings,
        };

        Ok(Some(cfg))
    }

    /// Implement slash commands so the user can run Start/Stop/Status/Tail directly from Zed's UI.
    /// This provides a reliable UI path (command palette / assistant slash commands) that will
    /// attempt to run and will produce textual output in the Assistant (visible to the user).
    fn run_slash_command(
        &self,
        command: zed::SlashCommand,
        args: Vec<String>,
        _worktree: Option<&zed::Worktree>,
    ) -> std::result::Result<zed::SlashCommandOutput, String> {
        let name = command.name.as_str();

        // Helper to parse simple key=value args (addr=..., token=...)
        fn parse_kv(args: &[String]) -> (String, String) {
            let mut addr = "127.0.0.1:8080".to_string();
            let mut token = "secret".to_string();
            for a in args.iter() {
                if let Some(v) = a.strip_prefix("addr=") {
                    addr = v.to_string();
                } else if let Some(v) = a.strip_prefix("token=") {
                    token = v.to_string();
                }
            }
            (addr, token)
        }

        match name {
            "start" => {
                let (addr, token) = parse_kv(&args);
                let (cmd_str, cmd_args, env) = if has_gnaw_binary() {
                    (
                        "gnawtreewriter".to_string(),
                        vec![
                            "mcp".into(),
                            "serve".into(),
                            "--addr".into(),
                            addr.clone(),
                            "--token".into(),
                            token.clone(),
                        ],
                        vec![],
                    )
                } else {
                    (
                        "./scripts/mcp-serve.sh".to_string(),
                        vec![
                            "--addr".into(),
                            addr.clone(),
                            "--token".into(),
                            token.clone(),
                        ],
                        vec![("MCP_TOKEN".to_string(), token.clone())],
                    )
                };

                let mut proc = zed::process::Command::new(cmd_str).args(cmd_args).envs(env);
                match proc.output() {
                    Ok(out) => {
                        let text = format!(
                            "Started server (stdout):\n{}\n\n(stderr):\n{}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                        Ok(zed::SlashCommandOutput {
                            sections: vec![zed::SlashCommandOutputSection {
                                range: (0..text.len()).into(),
                                label: "Start".to_string(),
                            }],
                            text,
                        })
                    }
                    Err(e) => Err(format!("Failed to start server: {}", e)),
                }
            }

            "stop" => {
                let mut proc = zed::process::Command::new("./scripts/mcp-stop.sh");
                match proc.output() {
                    Ok(out) => {
                        let text = format!(
                            "Stop result (stdout):\n{}\n\n(stderr):\n{}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                        Ok(zed::SlashCommandOutput {
                            sections: vec![zed::SlashCommandOutputSection {
                                range: (0..text.len()).into(),
                                label: "Stop".to_string(),
                            }],
                            text,
                        })
                    }
                    Err(e) => Err(format!("Failed to stop server: {}", e)),
                }
            }

            "status" => {
                let (addr, token) = parse_kv(&args);
                // Prefer gnawtreewriter binary if available, otherwise try curl initialize.
                if has_gnaw_binary() {
                    let mut proc = zed::process::Command::new("gnawtreewriter").args(vec![
                        "mcp".into(),
                        "status".into(),
                        "--url".into(),
                        format!("http://{}/", addr),
                        "--token".into(),
                        token.clone(),
                    ]);
                    match proc.output() {
                        Ok(out) => {
                            let text = format!(
                                "Status (stdout):\n{}\n\n(stderr):\n{}",
                                String::from_utf8_lossy(&out.stdout),
                                String::from_utf8_lossy(&out.stderr)
                            );
                            Ok(zed::SlashCommandOutput {
                                sections: vec![zed::SlashCommandOutputSection {
                                    range: (0..text.len()).into(),
                                    label: "Status".to_string(),
                                }],
                                text,
                            })
                        }
                        Err(e) => Err(format!("Status command failed: {}", e)),
                    }
                } else {
                    // Try a simple curl initialize call
                    let body = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;
                    let mut proc = zed::process::Command::new("curl").args(vec![
                        "-s".into(),
                        format!("http://{}/", addr),
                        "-H".into(),
                        format!("Authorization: Bearer {}", token),
                        "-H".into(),
                        "Content-Type: application/json".into(),
                        "-d".into(),
                        body.into(),
                    ]);
                    match proc.output() {
                        Ok(out) => {
                            let text = format!(
                                "Status (curl stdout):\n{}\n\n(stderr):\n{}",
                                String::from_utf8_lossy(&out.stdout),
                                String::from_utf8_lossy(&out.stderr)
                            );
                            Ok(zed::SlashCommandOutput {
                                sections: vec![zed::SlashCommandOutputSection {
                                    range: (0..text.len()).into(),
                                    label: "Status (curl)".to_string(),
                                }],
                                text,
                            })
                        }
                        Err(e) => Err(format!("Status (curl) failed: {}", e)),
                    }
                }
            }

            "tail_log" => {
                // Read the last ~200 lines of .mcp-server.log if present
                let path = std::path::Path::new(".mcp-server.log");
                if !path.exists() {
                    return Err(
                        "Log file '.mcp-server.log' not found in the project directory."
                            .to_string(),
                    );
                }
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let lines: Vec<&str> = content.lines().collect();
                        let last = if lines.len() > 200 {
                            lines[lines.len() - 200..].join("\n")
                        } else {
                            lines.join("\n")
                        };
                        let text = format!(
                            "Last {} lines of .mcp-server.log:\n\n{}",
                            std::cmp::min(200, lines.len()),
                            last
                        );
                        Ok(zed::SlashCommandOutput {
                            sections: vec![zed::SlashCommandOutputSection {
                                range: (0..text.len()).into(),
                                label: "Tail log".to_string(),
                            }],
                            text,
                        })
                    }
                    Err(e) => Err(format!("Failed to read log file: {}", e)),
                }
            }

            other => Err(format!("unknown slash command: \"{}\"", other)),
        }
    }
}

#[cfg(feature = "zed")]
zed::register_extension!(GnawExtension);

#[cfg(not(feature = "zed"))]
impl GnawExtension {
    /// Helper that returns a minimal representation of what would be run.
    /// This is useful for local testing of the extension logic when the `zed` crate is not available.
    pub fn context_server_command_preview(&self) -> (String, Vec<String>, Vec<(String, String)>) {
        preferred_server_invocation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_command_non_empty() {
        let ext = GnawExtension::new();
        let (cmd, args, _env) = ext.context_server_command_preview();
        assert!(!cmd.is_empty(), "expected a command to exist");
        assert!(!args.is_empty(), "expected some args to exist");
    }
}
