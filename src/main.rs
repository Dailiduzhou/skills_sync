// src/main.rs
mod app;
mod cli;
mod config;
mod git;

use clap::Parser;
use crate::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    app::run(cli).await
}
