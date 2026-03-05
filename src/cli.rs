use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skillsync", about = "多 Git 仓库状态同步与更新工具")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 检查所有仓库的状态
    Status,
    /// 更新所有落后的仓库
    Update,
    /// 查询当前并发数
    Concurrency,
    /// 设置并发数
    SetConcurrency {
        /// 并发数，必须大于 0
        value: usize,
    },
    /// 将一个本地 Git 仓库添加到监控列表中
    Add {
        /// 仓库的本地路径，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
    },
    /// 递归扫描目录，把子文件夹中的 Git 仓库加入监控列表
    AddRecursive {
        /// 扫描的根目录，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
        /// 最大扫描深度（不传则不限深度，0 表示仅当前目录）
        #[arg(long)]
        max_depth: Option<usize>,
    },
    /// 从监控列表中删除一个或多个本地 Git 仓库
    Remove {
        /// 仓库的本地路径，支持多个
        #[arg(num_args = 1..)]
        paths: Vec<String>,
    },
    /// 递归扫描目录，从监控列表中删除其下所有 Git 仓库
    RemoveRecursive {
        /// 扫描的根目录，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
        /// 最大扫描深度（不传则不限深度，0 表示仅当前目录）
        #[arg(long)]
        max_depth: Option<usize>,
    },
}
