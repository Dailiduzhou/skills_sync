use anyhow::{Context, Result};
use std::process::Command;

pub struct RepoStatus {
    pub path: String,
    pub ahead: u32,
    pub behind: u32,
}

/// 执行 git fetch
pub fn fetch_repo(repo_path: &str) -> Result<()> {
    let status = Command::new("git")
        .current_dir(repo_path) // 非常关键：告诉命令在哪个目录执行！
        .arg("fetch")
        .arg("--quiet") // 减少不必要的输出
        .status()
        .context(format!("在 {} 执行 git fetch 失败", repo_path))?;

    if !status.success() {
        anyhow::bail!("Git fetch 失败: {}", repo_path);
    }
    Ok(())
}

/// 获取落后/领先的 commits 数量
pub fn get_repo_status(repo_path: &str) -> Result<RepoStatus> {
    let output = Command::new("git")
        .current_dir(repo_path)
        // HEAD...@{u} 需要仓库设置了 upstream tracking
        .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
        .output()
        .context("无法执行 git rev-list")?;

    if !output.status.success() {
        // 如果没有 upstream，可能会报错。这里做简单处理
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("无法获取分支状态 (可能未绑定云端分支): {}", err);
    }

    // 解析输出，例如 "0\t4\n" -> ahead 0, behind 4
    let result_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = result_str.trim().split_whitespace().collect();

    if parts.len() == 2 {
        let ahead: u32 = parts[0].parse().unwrap_or(0);
        let behind: u32 = parts[1].parse().unwrap_or(0);
        Ok(RepoStatus {
            path: repo_path.to_string(),
            ahead,
            behind,
        })
    } else {
        anyhow::bail!("解析 git rev-list 输出失败: {}", result_str);
    }
}

/// 执行 git pull
pub fn update_repo(repo_path: &str) -> Result<()> {
    let status = Command::new("git")
        .current_dir(repo_path)
        .args(["pull", "--ff-only"])
        .status()?;

    if !status.success() {
        anyhow::bail!("拉取失败，可能存在冲突或本地未提交的修改");
    }
    Ok(())
}
