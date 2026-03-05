use clap::{Parser, Subcommand};
use colored::Colorize;
mod config;
use config::Config;
mod git_ops;

#[derive(Parser)]
#[command(name = "skills_sync", about = "多 Git 仓库状态同步与更新工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 检查所有仓库的状态
    Status,
    /// 更新所有落后的仓库
    Update,
    /// 将一个本地 Git 仓库添加到监控列表中
    Add {
        /// 仓库的本地路径，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 每次运行命令前，先加载配置文件
    let mut config = Config::load()?;

    match &cli.command {
        Commands::Add { path } => {
            // 调用我们写好的方法添加路径并保存
            config.add_repo(path.to_string())?;
        }
        Commands::Status => {
            if config.repos.is_empty() {
                println!("列表为空。请先使用 `gitsync add <路径>` 添加仓库。");
                return Ok(());
            }
            println!("正在检查 {} 个仓库...", config.repos.len());
            // 这里替换为上一次回答中的循环逻辑：
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
                println!("列表为空。请先使用 `gitsync add <路径>` 添加仓库。");
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
