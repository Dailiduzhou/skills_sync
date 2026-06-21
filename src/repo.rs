use anyhow::Result;
use std::collections::HashSet;
use tokio::fs;

use crate::config::Config;

#[derive(Debug, Default)]
pub struct AddReposSummary {
    pub added: usize,
    pub already: usize,
    pub failed: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub struct RemoveReposSummary {
    pub removed: usize,
    pub missing: usize,
    pub failed: Vec<(String, String)>,
}

pub async fn add_repo(config: &mut Config, repo_path: String) -> Result<()> {
    let summary = add_repos(config, vec![repo_path.clone()]).await?;
    if summary.added > 0 {
        println!("✅ 成功添加仓库: {}", repo_path);
    } else if summary.already > 0 {
        println!("⚠️ 仓库已存在，无需重复添加。");
    }
    if let Some((path, reason)) = summary.failed.first() {
        anyhow::bail!("无法添加仓库: {} ({})", path, reason);
    }
    Ok(())
}

pub async fn add_repos(config: &mut Config, repo_paths: Vec<String>) -> Result<AddReposSummary> {
    let mut summary = AddReposSummary::default();
    let mut seen = HashSet::new();

    for repo_path in repo_paths {
        if !seen.insert(repo_path.clone()) {
            continue;
        }

        let path = std::path::Path::new(&repo_path);
        let abs_path = match fs::canonicalize(path).await {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                summary
                    .failed
                    .push((repo_path, format!("找不到指定的路径: {}", e)));
                continue;
            }
        };

        if config.repos.contains(&abs_path) {
            summary.already += 1;
            continue;
        }

        config.repos.push(abs_path);
        summary.added += 1;
    }

    if summary.added > 0 {
        config.save().await?;
    }

    Ok(summary)
}

pub async fn remove_repos(
    config: &mut Config,
    repo_paths: Vec<String>,
) -> Result<RemoveReposSummary> {
    let mut summary = RemoveReposSummary::default();
    let mut seen = HashSet::new();

    for repo_path in repo_paths {
        if !seen.insert(repo_path.clone()) {
            continue;
        }

        let path = std::path::Path::new(&repo_path);
        let abs_path = match fs::canonicalize(path).await {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                summary
                    .failed
                    .push((repo_path, format!("找不到指定的路径: {}", e)));
                continue;
            }
        };

        let before = config.repos.len();
        config.repos.retain(|p| p != &abs_path);
        if config.repos.len() < before {
            summary.removed += 1;
        } else {
            summary.missing += 1;
        }
    }

    if summary.removed > 0 {
        config.save().await?;
    }

    Ok(summary)
}

