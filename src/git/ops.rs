use anyhow::{Context, Result};
use std::{path::Path, process::Stdio};
use tokio::process::Command;

pub struct RepoStatus {
    pub ahead: u32,
    pub behind: u32,
}

/// 执行 git fetch
pub async fn fetch_repo(repo_path: &str) -> Result<()> {
    let status = Command::new("git")
        .current_dir(repo_path) // 非常关键：告诉命令在哪个目录执行！
        .arg("fetch")
        .arg("--quiet") // 减少不必要的输出
        .status()
        .await
        .context(format!("在 {} 执行 git fetch 失败", repo_path))?;

    if !status.success() {
        anyhow::bail!("Git fetch 失败: {}", repo_path);
    }
    Ok(())
}

/// 获取落后/领先的 commits 数量
pub async fn get_repo_status(repo_path: &str) -> Result<RepoStatus> {
    let output = Command::new("git")
        .current_dir(repo_path)
        // HEAD...@{u} 需要仓库设置了 upstream tracking
        .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
        .output()
        .await
        .context("无法执行 git rev-list")?;

    if !output.status.success() {
        // 如果没有 upstream，可能会报错。这里做简单处理
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("无法获取分支状态 (可能未绑定云端分支): {}", err);
    }

    // 解析输出，例如 "0\t4\n" -> ahead 0, behind 4
    let result_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = result_str.split_whitespace().collect();

    if parts.len() == 2 {
        let ahead: u32 = parts[0].parse().unwrap_or(0);
        let behind: u32 = parts[1].parse().unwrap_or(0);
        Ok(RepoStatus { ahead, behind })
    } else {
        anyhow::bail!("解析 git rev-list 输出失败: {}", result_str);
    }
}

/// 执行 git pull
pub async fn update_repo(repo_path: &str) -> Result<()> {
    let status = Command::new("git")
        .current_dir(repo_path)
        .args(["pull", "--ff-only"])
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("拉取失败，可能存在冲突或本地未提交的修改");
    }
    Ok(())
}

/// 执行 git clone
pub async fn clone_repo(
    repo_url: &str,
    dest_dir: &Path,
    git_ssh_command: Option<&str>,
) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--quiet").arg(repo_url).arg(dest_dir);

    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    if let Some(command) = git_ssh_command {
        cmd.env("GIT_SSH_COMMAND", command);
    }

    let status = cmd
        .status()
        .await
        .with_context(|| format!("在 {} 执行 git clone 失败", dest_dir.display()))?;

    if !status.success() {
        anyhow::bail!("Git clone 失败: {}", repo_url);
    }
    Ok(())
}

/// 从仓库地址推断默认目录名
pub fn infer_repo_dir(repo_url: &str) -> String {
    let separators = ['/', '\\', ':'];

    let last_segment = repo_url
        .trim_end_matches(separators)
        .rsplit(separators)
        .next()
        .unwrap();

    let name = last_segment.strip_suffix(".git").unwrap_or(last_segment);

    if name.is_empty() { "repo" } else { name }.to_string()
}
