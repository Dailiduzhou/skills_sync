use anyhow::Result;
use colored::Colorize;
use futures::stream::{self, StreamExt};

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::git::ops as git_ops;
use crate::git::scan as git_scan;
use std::path::Path;

pub async fn run(cli: Cli) -> Result<()> {
    // 每次运行命令前，先加载配置文件
    let mut config = Config::load().await?;

    match cli.command {
        Commands::Concurrency => {
            println!("{}", config.get_concurrency());
        }
        Commands::SetConcurrency { value } => {
            config.set_concurrency(value).await?;
            println!("并发数已设置为 {}", value);
        }
        Commands::Add { path } => {
            // 调用我们写好的方法添加路径并保存
            config.add_repo(path).await?;
        }
        Commands::AddRecursive { path, max_depth } => {
            let root = Path::new(&path);
            let repos = git_scan::find_git_repos(root, max_depth).await?;

            if repos.is_empty() {
                println!("未找到 Git 仓库: {}", root.display());
                return Ok(());
            }

            let repo_paths: Vec<String> = repos
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            let summary = config.add_repos(repo_paths).await?;
            println!(
                "共发现 {} 个仓库，新增 {} 个，已存在 {} 个。",
                repos.len(),
                summary.added,
                summary.already
            );

            if !summary.failed.is_empty() {
                println!("以下路径添加失败:");
                for (path, reason) in summary.failed {
                    println!("  - {} ({})", path, reason);
                }
            }
        }
        Commands::Remove { paths } => {
            let summary = config.remove_repos(paths).await?;
            println!(
                "删除完成：移除 {} 个，未找到 {} 个。",
                summary.removed, summary.missing
            );

            if !summary.failed.is_empty() {
                println!("以下路径删除失败:");
                for (path, reason) in summary.failed {
                    println!("  - {} ({})", path, reason);
                }
            }
        }
        Commands::RemoveRecursive { path, max_depth } => {
            let root = Path::new(&path);
            let repos = git_scan::find_git_repos(root, max_depth).await?;

            if repos.is_empty() {
                println!("未找到 Git 仓库: {}", root.display());
                return Ok(());
            }

            let repo_paths: Vec<String> = repos
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            let summary = config.remove_repos(repo_paths).await?;
            println!(
                "共发现 {} 个仓库，移除 {} 个，未找到 {} 个。",
                repos.len(),
                summary.removed,
                summary.missing
            );

            if !summary.failed.is_empty() {
                println!("以下路径删除失败:");
                for (path, reason) in summary.failed {
                    println!("  - {} ({})", path, reason);
                }
            }
        }
        Commands::Status => {
            if config.repos.is_empty() {
                println!(
                    "列表为空。请先使用 `skillsync add <路径>` 或 `skillsync add-recursive <路径>` 添加仓库。"
                );
                return Ok(());
            }
            if config.get_concurrency() == 0 {
                anyhow::bail!("并发数必须大于 0，请使用 `skillsync set-concurrency <N>` 设置");
            }
            println!("正在检查 {} 个仓库...", config.repos.len());
            let concurrency = config.get_concurrency();
            let repos: Vec<String> = config.repos.clone();

            let mut results = stream::iter(repos.into_iter().enumerate())
                .map(|(idx, repo)| async move {
                    let mut lines = Vec::new();
                    lines.push(format!("检查路径: {}", repo));

                    if let Err(e) = git_ops::fetch_repo(&repo).await {
                        lines.push(format!("❌ [{}]: {}", repo.red(), e));
                        return (idx, lines);
                    }

                    match git_ops::get_repo_status(&repo).await {
                        Ok(status) => {
                            if status.behind > 0 {
                                lines.push(format!(
                                    "⚠️  [{}]: 落后云端 {} 个 commits",
                                    repo.yellow(),
                                    status.behind.to_string().red()
                                ));
                            } else if status.ahead > 0 {
                                lines.push(format!(
                                    "🚀 [{}]: 领先云端 {} 个 commits (未 push)",
                                    repo.cyan(),
                                    status.ahead
                                ));
                            } else {
                                lines.push(format!("✅ [{}]: 已是最新", repo.green()));
                            }
                        }
                        Err(e) => lines.push(format!("❌ [{}]: {}", repo.red(), e)),
                    }

                    (idx, lines)
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;

            results.sort_by_key(|(idx, _)| *idx);
            for (_, lines) in results {
                for line in lines {
                    println!("{}", line);
                }
            }
        }
        Commands::Update => {
            if config.repos.is_empty() {
                println!(
                    "列表为空。请先使用 `skillsync add <路径>` 或 `skillsync add-recursive <路径>` 添加仓库。"
                );
                return Ok(());
            }
            if config.get_concurrency() == 0 {
                anyhow::bail!("并发数必须大于 0，请使用 `skillsync set-concurrency <N>` 设置");
            }
            let concurrency = config.get_concurrency();
            let repos: Vec<String> = config.repos.clone();

            let mut results = stream::iter(repos.into_iter().enumerate())
                .map(|(idx, repo)| async move {
                    let mut lines = Vec::new();
                    lines.push(format!("更新路径: {}", repo));

                    if let Err(e) = git_ops::fetch_repo(&repo).await {
                        lines.push(format!("❌ [{}]: {}", repo.red(), e));
                        return (idx, lines);
                    }

                    match git_ops::get_repo_status(&repo).await {
                        Ok(status) => {
                            if status.behind > 0 {
                                let result = match git_ops::update_repo(&repo).await {
                                    Ok(_) => format!("{}", "成功!".green()),
                                    Err(e) => format!("{} ({})", "失败".red(), e),
                                };
                                lines.push(format!("🔄 正在更新 {} ... {}", repo.yellow(), result));
                            } else {
                                lines.push(format!("✅ [{}]: 无需更新", repo.green()));
                            }
                        }
                        Err(_) => {}
                    }

                    (idx, lines)
                })
                .buffer_unordered(concurrency)
                .collect::<Vec<_>>()
                .await;

            results.sort_by_key(|(idx, _)| *idx);
            for (_, lines) in results {
                for line in lines {
                    println!("{}", line);
                }
            }
        }
    }

    Ok(())
}
