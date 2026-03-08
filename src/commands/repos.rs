use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::git::scan as git_scan;

pub async fn add(config: &mut Config, path: String) -> Result<()> {
    config.add_repo(path).await?;
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

    Ok(())
}

pub async fn remove(config: &mut Config, paths: Vec<String>) -> Result<()> {
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

    Ok(())
}

pub async fn remove_recursive(
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

    Ok(())
}
