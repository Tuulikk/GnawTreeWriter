//! Zed extension for GnawTreeWriter using MCP over Stdio.
use zed_extension_api as zed;

pub struct GnawExtension {}

impl GnawExtension {
    pub fn new() -> Self {
        Self {}
    }
}

impl zed::Extension for GnawExtension {
    fn new() -> Self {
        GnawExtension {}
    }

    /// Return the command Zed should run to start the context server.
    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> std::result::Result<zed::process::Command, String> {
        // We use the 'gnawtreewriter' binary from PATH.
        // The user should have the binary installed.
        Ok(zed::process::Command {
            command: "gnawtreewriter".to_string(),
            args: vec!["mcp".into(), "stdio".into()],
            env: Vec::new(),
        })
    }
}

zed::register_extension!(GnawExtension);