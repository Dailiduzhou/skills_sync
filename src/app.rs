use anyhow::Result;

use crate::cli::{Cli, Commands};
use crate::commands::{clone as clone_cmd, concurrency, key, repos, status, update};
use crate::config::Config;

pub async fn run(cli: Cli) -> Result<()> {
    // 每次运行命令前，先加载配置文件
    let mut config = Config::load().await?;

    match cli.command {
        Commands::Concurrency => concurrency::show(&config)?,
        Commands::SetConcurrency { value } => {
            concurrency::set(&mut config, value).await?;
        }
        Commands::Add { paths } => {
            repos::add(&mut config, paths).await?;
        }
        Commands::AddRecursive { path, max_depth } => {
            repos::add_recursive(&mut config, path, max_depth).await?;
        }
        Commands::Remove { paths } => {
            repos::remove(&mut config, paths).await?;
        }
        Commands::RemoveRecursive { path, max_depth } => {
            repos::remove_recursive(&mut config, path, max_depth).await?;
        }
        Commands::Clone { repos, dir } => {
            clone_cmd::run_clone(&mut config, repos, dir).await?;
        }
        Commands::Key { command } => {
            key::run(command)?;
        }
        Commands::Status => {
            status::run(&config).await?;
        }
        Commands::Update => {
            update::run(&config).await?;
        }
    }

    Ok(())
}
