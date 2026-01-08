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

use anyhow::Result;
use std::collections::HashMap;

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
fn preferred_server_invocation() -> (String, Vec<String>, HashMap<String, String>) {
    // Default settings (simple defaults for local dev)
    const ADDR: &str = "127.0.0.1:8080";
    const TOKEN: &str = "secret";

    // If the `which` crate is available, prefer the binary when installed
    // (This is a best-effort convenience. If `which` is not available in your workspace
    // add it to the extension crate's Cargo.toml: `which = "4"`).
    if which::which("gnawtreewriter").is_ok() {
        let mut args = Vec::new();
        args.push("mcp".into());
        args.push("serve".into());
        args.push("--addr".into());
        args.push(ADDR.into());
        args.push("--token".into());
        args.push(TOKEN.into());
        (String::from("gnawtreewriter"), args, HashMap::new())
    } else {
        // Use shell wrapper script as fallback (portable for development)
        let cmdline = format!("./scripts/mcp-serve.sh --addr {} --token {}", ADDR, TOKEN);
        let args = vec!["-c".into(), cmdline];
        let mut env = HashMap::new();
        // Optionally expose the token via env (some users prefer env var over CLI arg)
        env.insert("MCP_TOKEN".to_string(), TOKEN.to_string());
        (String::from("sh"), args, env)
    }
}

#[cfg(feature = "zed")]
impl zed::Extension for GnawExtension {
    /// Return the command Zed should run to start the context server.
    ///
    /// The exact signature and returned type should match the `zed` crate you are using.
    /// The example below follows the conceptual API described in the Zed docs:
    /// `context_server_command(...) -> Result<zed::Command>`.
    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> Result<zed::Command> {
        let (command, args, env_map) = preferred_server_invocation();

        // Convert env HashMap into the shape the zed::Command expects.
        // The real `zed::Command` type varies with zed crate versions — adapt as needed.
        let env: Vec<(String, String)> = env_map.into_iter().collect();

        // Construct and return the command that Zed will run to start the MCP server.
        // If the `zed::Command` struct in your version uses a different shape, adjust fields accordingly.
        Ok(zed::Command { command, args, env })
    }
}

#[cfg(not(feature = "zed"))]
impl GnawExtension {
    /// Helper that returns a minimal representation of what would be run.
    /// This is useful for local testing of the extension logic when the `zed` crate is not available.
    pub fn context_server_command_preview(&self) -> (String, Vec<String>, HashMap<String, String>) {
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
