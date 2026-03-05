use anyhow::Result;
use colored::Colorize;

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::git::ops as git_ops;

pub fn run(cli: Cli) -> Result<()> {
    // 每次运行命令前，先加载配置文件
    let mut config = Config::load()?;

    match cli.command {
        Commands::Add { path } => {
            // 调用我们写好的方法添加路径并保存
            config.add_repo(path)?;
        }
        Commands::Status => {
            if config.repos.is_empty() {
                println!("列表为空。请先使用 `skillsync add <路径>` 添加仓库。");
                return Ok(());
            }
            println!("正在检查 {} 个仓库...", config.repos.len());
            for repo in &config.repos {
                println!("检查路径: {}", repo);
                git_ops::fetch_repo(repo)?;
                match git_ops::get_repo_status(repo) {
                    Ok(status) => {
                        if status.behind > 0 {
                            println!(
                                "⚠️  [{}]: 落后云端 {} 个 commits",
                                repo.yellow(),
                                status.behind.to_string().red()
                            );
                        } else if status.ahead > 0 {
                            println!(
                                "🚀 [{}]: 领先云端 {} 个 commits (未 push)",
                                repo.cyan(),
                                status.ahead
                            );
                        } else {
                            println!("✅ [{}]: 已是最新", repo.green());
                        }
                    }
                    Err(e) => eprintln!("❌ [{}]: {}", repo.red(), e),
                }
            }
        }
        Commands::Update => {
            if config.repos.is_empty() {
                println!("列表为空。请先使用 `skillsync add <路径>` 添加仓库。");
                return Ok(());
            }
            for repo in &config.repos {
                println!("更新路径: {}", repo);
                git_ops::fetch_repo(repo)?;
                if let Ok(status) = git_ops::get_repo_status(repo) {
                    if status.behind > 0 {
                        print!("🔄 正在更新 {} ... ", repo.yellow());
                        match git_ops::update_repo(repo) {
                            Ok(_) => println!("{}", "成功!".green()),
                            Err(e) => println!("{} ({})", "失败".red(), e),
                        }
                    } else {
                        println!("✅ [{}]: 无需更新", repo.green());
                    }
                }
            }
        }
    }

    Ok(())
}
