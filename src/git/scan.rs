use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::{sync::Semaphore, task::JoinSet};

// 使用枚举让异步任务的返回结果语义更加清晰
enum DirResult {
    /// 找到了 Git 仓库，返回其路径
    Repo(PathBuf),
    /// 不是仓库，返回需要继续扫描的子目录及其深度
    Subdirs(Vec<(PathBuf, usize)>),
}

/// 递归扫描目录，返回所有 Git 仓库的根目录路径
pub async fn find_git_repos(root: &Path, max_depth: Option<usize>) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();
    let mut join_set = JoinSet::new();

    // 限制最大并发数为 100，防止耗尽操作系统的文件句柄
    let semaphore = Arc::new(Semaphore::new(100));

    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::with_template("{spinner} 扫描中... 已扫描 {pos} 个目录，发现 {msg} 个仓库")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    progress.enable_steady_tick(Duration::from_millis(120));
    progress.set_message("0");

    let mut scanned_dirs: u64 = 0;
    let mut found_repos: u64 = 0;

    // 派发初始的根目录任务
    join_set.spawn(process_dir(
        root.to_path_buf(),
        0,
        max_depth,
        semaphore.clone(),
    ));

    // 并发收集任务结果
    while let Some(res) = join_set.join_next().await {
        scanned_dirs += 1;
        let mut should_update_ui = false;
        match res {
            Ok(Ok(DirResult::Repo(path))) => {
                // 收到仓库路径，直接记录
                repos.push(path);
                found_repos += 1;
                should_update_ui = true;
            }
            Ok(Ok(DirResult::Subdirs(subdirs))) => {
                // 收到子目录列表，将其全部作为新任务派发出去并发执行
                for (sub_dir, depth) in subdirs {
                    join_set.spawn(process_dir(sub_dir, depth, max_depth, semaphore.clone()));
                }
            }
            // 忽略读取错误（如权限不足）或 Tokio 任务本身的 Panic
            _ => {}
        }
        if should_update_ui || scanned_dirs.is_multiple_of(50) {
            progress.set_position(scanned_dirs);
            progress.set_message(found_repos.to_string());
        }
    }

    progress.finish_and_clear();

    Ok(repos)
}

// 独立的异步工作函数，负责扫描单个目录
async fn process_dir(
    dir: PathBuf,
    depth: usize,
    max_depth: Option<usize>,
    semaphore: Arc<Semaphore>,
) -> Result<DirResult> {
    // 深度检查
    if let Some(limit) = max_depth
        && depth > limit
    {
        return Ok(DirResult::Subdirs(vec![]));
    }

    // 在执行 I/O 前获取信号量许可。
    // 如果当前并发数已达 100，任务会在这里挂起等待，直到有其他任务完成。
    let _permit = semaphore.acquire().await.unwrap();

    let mut read_dir = match fs::read_dir(&dir).await {
        Ok(it) => it,
        Err(_) => {
            // 这样整个 task 依然是 Ok 状态，不需要向外抛出 Error
            return Ok(DirResult::Subdirs(vec![]));
        }
    };
    let mut is_repo = false;
    let mut pending_subdirs = Vec::new();

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        if entry.file_name() == ".git" {
            is_repo = true;
            break; // 确认是仓库后立即终止扫描
        }

        let file_type = match entry.file_type().await {
            Ok(it) => it,
            Err(_) => continue,
        };

        if file_type.is_dir() && !file_type.is_symlink() {
            pending_subdirs.push((entry.path(), depth + 1));
        }
    }

    // 许可 (_permit) 会在函数结束时随着作用域自动释放 (Drop)

    if is_repo {
        Ok(DirResult::Repo(dir))
    } else {
        Ok(DirResult::Subdirs(pending_subdirs))
    }
}
