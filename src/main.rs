// src/main.rs
mod app;
mod cli;
mod config;
mod git;

use clap::Parser;
use crate::cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    app::run(cli)
}
