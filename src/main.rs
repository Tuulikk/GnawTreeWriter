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
            // THE HELPFUL GUARD: Intercept parsing errors to provide tips
            eprintln!("{}", e);
            
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("unrecognized subcommand") {
                eprintln!("\n💡 [GnawTip]: Not sure which command to use? Try 'gnawtreewriter wizard' or 'gnawtreewriter examples'.");
            } else if err_str.contains("required arguments were not provided") {
                eprintln!("\n💡 [GnawTip]: Every surgical edit needs a target. Use 'gnawtreewriter list <file>' to find node paths.");
            }
            
            std::process::exit(1);
        }
    };

    if let Err(err) = cli.run().await {
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
        
        std::process::exit(1);
    }
    
    Ok(())
}