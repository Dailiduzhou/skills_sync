use anyhow::Result;
use colored::Colorize;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};

use crate::config::Config;
use crate::git::ops as git_ops;

pub async fn run(config: &Config) -> Result<()> {
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
    let progress = ProgressBar::new(repos.len() as u64);
    progress.set_style(
        ProgressStyle::with_template("更新进度 [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("准备中");
    let progress_for_tasks = progress.clone();

    let mut results = stream::iter(repos.into_iter().enumerate())
        .map(|(idx, repo)| {
            let progress = progress_for_tasks.clone();
            async move {
                let mut lines = Vec::new();
                lines.push(format!("更新路径: {}", repo));

                if let Err(e) = git_ops::fetch_repo(&repo).await {
                    lines.push(format!("❌ [{}]: {}", repo.red(), e));
                    progress.inc(1);
                    return (idx, lines);
                }

                if let Ok(status) = git_ops::get_repo_status(&repo).await {
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
