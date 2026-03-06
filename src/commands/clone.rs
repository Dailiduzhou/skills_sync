use anyhow::Result;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

use crate::config::Config;
use crate::git::ops as git_ops;
use crate::ssh_key;

pub async fn run_clone(config: &mut Config, repos: Vec<String>, dir: String) -> Result<()> {
    let base_dir = PathBuf::from(&dir);
    if base_dir.exists() {
        if !base_dir.is_dir() {
            anyhow::bail!("目标路径不是目录: {}", base_dir.display());
        }
    } else {
        fs::create_dir_all(&base_dir)
            .await
            .map_err(|e| anyhow::anyhow!("无法创建目录 {}: {}", base_dir.display(), e))?;
    }

    let base_dir = fs::canonicalize(&base_dir)
        .await
        .map_err(|e| anyhow::anyhow!("无法解析目录 {}: {}", base_dir.display(), e))?;

    let config_path = Config::get_path().await?;
    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("无法定位配置目录"))?;
    let prepared_key = ssh_key::prepare_git_ssh_command(config_dir)?;
    let git_ssh_command = prepared_key
        .as_ref()
        .map(|prepared| prepared.ssh_command.as_str());

    if config.get_concurrency() == 0 {
        anyhow::bail!("并发数必须大于 0，请使用 `skillsync set-concurrency <N>` 设置");
    }

    let progress: ProgressBar = ProgressBar::new(repos.len() as u64);
    progress.set_style(
        ProgressStyle::with_template("克隆进度 [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    progress.set_message("准备中");

    let mut cloned_paths = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();

    let mut planned = Vec::new();
    let mut seen_dest = HashSet::new();

    for (idx, repo) in repos.into_iter().enumerate() {
        progress.set_message(repo.clone());
        let repo_dir = git_ops::infer_repo_dir(&repo);
        let dest = base_dir.join(&repo_dir);
        let dest_key = dest.to_string_lossy().to_string();

        if !seen_dest.insert(dest_key) {
            failures.push((repo, "目标目录冲突（同名仓库）".to_string()));
            progress.inc(1);
            continue;
        }

        if fs::metadata(&dest).await.is_ok() {
            failures.push((repo, format!("目标路径已存在: {}", dest.display())));
            progress.inc(1);
            continue;
        }

        planned.push((idx, repo, dest));
    }

    let concurrency = config.get_concurrency();
    let progress_for_tasks = progress.clone();
    let git_ssh_command = git_ssh_command.map(|value| value.to_string());

    let mut results: Vec<(
        usize,
        String,
        PathBuf,
        std::result::Result<(), anyhow::Error>,
    )> = stream::iter(planned.into_iter())
        .map(|(idx, repo, dest)| {
            let progress = progress_for_tasks.clone();
            let git_ssh_command = git_ssh_command.clone();
            async move {
                progress.set_message(repo.clone());
                let result = git_ops::clone_repo(&repo, &dest, git_ssh_command.as_deref()).await;
                progress.inc(1);
                (idx, repo, dest, result)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    results.sort_by_key(|(idx, _, _, _)| *idx);
    for (_, repo, dest, result) in results {
        match result {
            Ok(_) => {
                cloned_paths.push(dest.to_string_lossy().to_string());
            }
            Err(e) => {
                failures.push((repo, e.to_string()));
            }
        }
    }

    progress.finish_and_clear();

    if !cloned_paths.is_empty() {
        let summary = config.add_repos(cloned_paths).await?;
        println!(
            "克隆完成：新增 {} 个，已存在 {} 个。",
            summary.added, summary.already
        );
        if !summary.failed.is_empty() {
            println!("以下路径加入配置失败:");
            for (path, reason) in summary.failed {
                println!("  - {} ({})", path, reason);
            }
        }
    } else {
        println!("未克隆成功任何仓库。");
    }

    if !failures.is_empty() {
        println!("以下仓库克隆失败:");
        for (repo, reason) in failures {
            println!("  - {} ({})", repo, reason);
        }
    }

    Ok(())
}
