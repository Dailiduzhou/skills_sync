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
    /// 将一个本地 Git 仓库添加到监控列表中
    Add {
        /// 仓库的本地路径，默认为当前目录
        #[arg(default_value = ".")]
        path: String,
    },
}
