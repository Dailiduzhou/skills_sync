use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::git::scan as git_scan;
use crate::repo;

pub async fn add(config: &mut Config, paths: Vec<String>) -> Result<()> {
    if paths.len() == 1 {
        let path = paths.into_iter().next().unwrap();
        repo::add_repo(config, path).await?;
        return Ok(());
    }

    let summary = repo::add_repos(config, paths).await?;
    println!(
        "添加完成：新增 {} 个，已存在 {} 个。",
        summary.added, summary.already
    );

    if !summary.failed.is_empty() {
        println!("以下路径添加失败:");
        for (path, reason) in summary.failed {
            println!("  - {} ({})", path, reason);
        }
    }

    Ok(())
}

pub async fn add_recursive(
    config: &mut Config,
    path: String,
    max_depth: Option<usize>,
) -> Result<()> {
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

    let summary = repo::add_repos(config, repo_paths).await?;
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

    Ok(())
}

