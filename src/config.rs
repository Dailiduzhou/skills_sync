use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

/// 定义配置文件的结构
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    // 存放所有 Git 仓库的绝对路径
    pub repos: Vec<String>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

fn default_concurrency() -> usize {
    20
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repos: Vec::new(),
            concurrency: default_concurrency(),
        }
    }
}

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

impl Config {
    /// 获取配置文件的完整路径
    /// 例如 Linux 下可能是 ~/.config/skillsync/config.toml
    pub async fn get_path() -> Result<PathBuf> {
        // ProjectDirs::from(qualifier, organization, application)
        if let Some(proj_dirs) = ProjectDirs::from("com", "dailiduzhou", "skillsync") {
            let config_dir = proj_dirs.config_dir();

            // 如果目录不存在，自动创建它 (例如 ~/.config/skillsync/)
            if !config_dir.exists() {
                fs::create_dir_all(config_dir)
                    .await
                    .context("无法创建配置目录")?;
            }

            Ok(config_dir.join("config.toml"))
        } else {
            anyhow::bail!("无法确定操作系统的标准配置目录");
        }
    }

    /// 从文件加载配置
    pub async fn load() -> Result<Self> {
        let path = Self::get_path().await?;

        // 如果配置文件不存在，返回一个空的默认配置
        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("无法读取配置文件: {:?}", path))?;

        let config: Config =
            toml::from_str(&content).with_context(|| format!("配置文件格式错误: {:?}", path))?;

        Ok(config)
    }

    /// 将当前配置保存到文件
    pub async fn save(&self) -> Result<()> {
        let path = Self::get_path().await?;

        // 将 Rust 结构体序列化为 TOML 格式的字符串
        let toml_string = toml::to_string_pretty(self).context("序列化配置失败")?;

        fs::write(&path, toml_string)
            .await
            .with_context(|| format!("无法写入配置文件: {:?}", path))?;

        Ok(())
    }

    pub fn get_concurrency(&self) -> usize {
        self.concurrency
    }

    pub async fn set_concurrency(&mut self, value: usize) -> Result<()> {
        if value == 0 {
            anyhow::bail!("并发数必须大于 0");
        }
        self.concurrency = value;
        self.save().await?;
        Ok(())
    }

    /// 添加一个新的仓库路径并去重保存
    pub async fn add_repo(&mut self, repo_path: String) -> Result<()> {
        let summary = self.add_repos(vec![repo_path.clone()]).await?;
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

    /// 批量添加仓库路径，去重后保存一次
    pub async fn add_repos(&mut self, repo_paths: Vec<String>) -> Result<AddReposSummary> {
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

            if self.repos.contains(&abs_path) {
                summary.already += 1;
                continue;
            }

            self.repos.push(abs_path);
            summary.added += 1;
        }

        if summary.added > 0 {
            self.save().await?;
        }

        Ok(summary)
    }

    /// 批量删除仓库路径，去重后保存一次
    pub async fn remove_repos(&mut self, repo_paths: Vec<String>) -> Result<RemoveReposSummary> {
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

            let before = self.repos.len();
            self.repos.retain(|p| p != &abs_path);
            if self.repos.len() < before {
                summary.removed += 1;
            } else {
                summary.missing += 1;
            }
        }

        if summary.removed > 0 {
            self.save().await?;
        }

        Ok(summary)
    }
}
