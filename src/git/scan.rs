use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 递归扫描目录，返回所有 Git 仓库的根目录路径
pub async fn find_git_repos(root: &Path, max_depth: Option<usize>) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        if let Some(limit) = max_depth
            && depth > limit
        {
            continue;
        }

        let read_dir = match fs::read_dir(&dir).await {
            Ok(it) => it,
            Err(_) => {
                // 无法读取的目录直接跳过
                continue;
            }
        };

        let mut is_repo = false;

        let mut read_dir = read_dir;
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let file_name = entry.file_name();
            if file_name == ".git" {
                is_repo = true;
                continue;
            }

            let file_type = match entry.file_type().await {
                Ok(it) => it,
                Err(_) => continue,
            };

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                stack.push((entry.path(), depth + 1));
            }
        }

        if is_repo {
            repos.push(dir);
        }
    }

    repos.sort();
    repos.dedup();
    Ok(repos)
}
