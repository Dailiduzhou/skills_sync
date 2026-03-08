use anyhow::Result;
use colored::Colorize;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::Config;
use crate::git::ops as git_ops;

pub async fn run(config: &Config) -> Result<()> {
    if config.repos.is_empty() {
        println!(
            "列表为空。请先使用 `skillsync add <路径...>` 或 `skillsync add-recursive <路径>` 添加仓库。"
        );
        return Ok(());
    }
    if config.get_concurrency() == 0 {
        anyhow::bail!("并发数必须大于 0，请使用 `skillsync set-concurrency <N>` 设置");
    }

    println!("正在检查 {} 个仓库...", config.repos.len());
    let concurrency = config.get_concurrency();
    let repos: Vec<String> = config.repos.clone();
    let progress = ProgressBar::new(repos.len() as u64);
    progress.set_style(
        ProgressStyle::with_template("检查进度 [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("准备中");
    let progress_for_tasks = progress.clone();

    let mut results: Vec<(usize, Vec<String>)> = stream::iter(repos.into_iter().enumerate())
        .map(|(idx, repo)| {
            let progress = progress_for_tasks.clone();
            async move {
                let mut lines: Vec<String> = Vec::new();
                lines.push(format!("检查路径: {}", repo));

                if let Err(e) = git_ops::fetch_repo(&repo).await {
                    lines.push(format!("❌ [{}]: {}", repo.red(), e));
                    progress.inc(1);
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

                progress.inc(1);
                (idx, lines)
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    progress.finish_and_clear();
    results.sort_by_key(|(idx, _)| *idx);
    for (_, lines) in results {
        for line in lines {
            println!("{}", line);
        }
    }

    Ok(())
}
