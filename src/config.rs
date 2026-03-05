use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 定义配置文件的结构
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    // 存放所有 Git 仓库的绝对路径
    pub repos: Vec<String>,
}

impl Config {
    /// 获取配置文件的完整路径
    /// 例如 Linux 下可能是 ~/.config/gitsync/config.toml
    pub fn get_path() -> Result<PathBuf> {
        // ProjectDirs::from(qualifier, organization, application)
        if let Some(proj_dirs) = ProjectDirs::from("com", "myname", "gitsync") {
            let config_dir = proj_dirs.config_dir();

            // 如果目录不存在，自动创建它 (例如 ~/.config/gitsync/)
            if !config_dir.exists() {
                fs::create_dir_all(config_dir).context("无法创建配置目录")?;
            }

            Ok(config_dir.join("config.toml"))
        } else {
            anyhow::bail!("无法确定操作系统的标准配置目录");
        }
    }

    /// 从文件加载配置
    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;

        // 如果配置文件不存在，返回一个空的默认配置
        if !path.exists() {
            return Ok(Config::default());
        }

        let content =
            fs::read_to_string(&path).with_context(|| format!("无法读取配置文件: {:?}", path))?;

        let config: Config =
            toml::from_str(&content).with_context(|| format!("配置文件格式错误: {:?}", path))?;

        Ok(config)
    }

    /// 将当前配置保存到文件
    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;

        // 将 Rust 结构体序列化为 TOML 格式的字符串
        let toml_string = toml::to_string_pretty(self).context("序列化配置失败")?;

        fs::write(&path, toml_string).with_context(|| format!("无法写入配置文件: {:?}", path))?;

        Ok(())
    }

    /// 添加一个新的仓库路径并去重保存
    pub fn add_repo(&mut self, repo_path: String) -> Result<()> {
        let path = std::path::Path::new(&repo_path);

        // 获取绝对路径，确保存储的是标准路径
        let abs_path = fs::canonicalize(path)
            .context(format!("找不到指定的路径: {}", repo_path))?
            .to_string_lossy()
            .to_string();

        if !self.repos.contains(&abs_path) {
            self.repos.push(abs_path);
            self.save()?;
            println!("✅ 成功添加仓库: {}", repo_path);
        } else {
            println!("⚠️ 仓库已存在，无需重复添加。");
        }

        Ok(())
    }
}
