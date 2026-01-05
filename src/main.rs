// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Result;
use clap::Parser;
use gnawtreewriter::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    Cli::parse().run().await
}
