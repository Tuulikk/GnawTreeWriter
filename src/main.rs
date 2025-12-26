pub mod cli;
pub mod core;
pub mod parser;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::parse().run().await
}
