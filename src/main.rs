// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Result;
use clap::Parser;
use gnawtreewriter::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            // Help and version are NOT failures. Mapping every parse error to
            // `exit(1)` made `gnawtreewriter --version` look like a crash:
            // `gtw --version || echo missing` reported "missing" on a working
            // install, and `--help` text went to stderr instead of stdout.
            // `e.exit()` uses clap's own conventions: help/version -> stdout +
            // exit 0, real usage errors -> stderr + exit 2.
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    e.exit()
                }
                _ => {
                    // THE HELPFUL GUARD: tips for the common mistakes.
                    eprintln!("{}", e);

                    let err_str = e.to_string().to_lowercase();
                    if err_str.contains("unrecognized subcommand") {
                        eprintln!("\n💡 [GnawTip]: Not sure which command to use? Try 'gnawtreewriter wizard' or 'gnawtreewriter examples'.");
                    } else if err_str.contains("required arguments were not provided") {
                        eprintln!("\n💡 [GnawTip]: Every surgical edit needs a target. Use 'gnawtreewriter list <file>' to find node paths.");
                    }

                    // 2 = usage error, so a caller can distinguish "I invoked it
                    // wrongly" from "the tool failed" (1).
                    std::process::exit(2);
                }
            }
        }
    };

    if let Err(err) = cli.run().await {
        // Check if JSON mode was requested
        let json_mode = std::env::var("GNAW_JSON").is_ok();
        if json_mode {
            use gnawtreewriter::core::diagnostics::{DiagnosticError, DiagnosticReport};
            let diag = DiagnosticReport::err(
                "gnawtreewriter",
                "",
                "unknown",
                DiagnosticError::from_anyhow(&err, "unknown"),
            );
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&diag)
                    .unwrap_or_else(|_| format!("{{\"success\": false, \"error\": \"{}\"}}", err))
            );
        } else {
            eprintln!("Error: {}", err);

            // Logical tips based on execution errors
            let err_msg = err.to_string().to_lowercase();
            if err_msg.contains("guardian block") {
                eprintln!("\n🛡️  [GuardianTip]: Significant code loss detected. Use --force if this deletion is intentional.");
            } else if err_msg.contains("syntax error") {
                eprintln!("\n✨ [DuplexTip]: The proposed edit broke the AST. GnawTreeWriter prevented this to keep your project stable.");
            } else if err_msg.contains("modernbert") {
                eprintln!("\n🧠 [AiTip]: Semantic features require ModernBERT. Run 'gnawtreewriter ai setup' or check your features.");
            }
        }

        std::process::exit(1);
    }

    Ok(())
}
