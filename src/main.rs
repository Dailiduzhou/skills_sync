use clap::Parser;
use skills_sync::{app, cli::Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    app::run(cli).await
}
