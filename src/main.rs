// src/main.rs
mod app;
mod cli;
mod commands;
mod config;
mod git;
mod ssh_key;

use clap::Parser;
use crate::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    app::run(cli).await
}
