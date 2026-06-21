use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{Builder, TempPath};

use super::keyring::{self, KeyStorage};

pub struct PreparedKey {
    pub ssh_command: String,
    pub _temp_key: Option<TempPath>,
}

pub fn prepare_git_ssh_command(config_dir: &Path) -> Result<Option<PreparedKey>> {
    let stored_key = match keyring::get_key()? {
        Some(value) => value,
        None => return Ok(None),
    };
    if stored_key.storage == KeyStorage::Fallback {
        eprintln!(
            "提示：正在使用配置文件中的回退私钥（弱加密）。建议恢复系统钥匙串或使用 ssh-agent。"
        );
    }

    let known_hosts = ensure_known_hosts(config_dir)?;
    let known_hosts_path = quote_path(&known_hosts);

    if try_add_key_to_agent(config_dir, &stored_key.value)? {
        eprintln!("已将私钥加载到 ssh-agent，将优先使用 ssh-agent 完成认证。");
        let ssh_command = format!(
            "ssh -o UserKnownHostsFile={} -o StrictHostKeyChecking=accept-new",
            known_hosts_path
        );
        return Ok(Some(PreparedKey {
            ssh_command,
            _temp_key: None,
        }));
    }

    let temp_key = write_temp_key(config_dir, &stored_key.value)?;
    let key_path = quote_path(temp_key.as_ref());

    let ssh_command = format!(
        "ssh -i {} -o IdentitiesOnly=yes -o UserKnownHostsFile={} -o StrictHostKeyChecking=accept-new",
        key_path, known_hosts_path
    );

    Ok(Some(PreparedKey {
        ssh_command,
        _temp_key: Some(temp_key),
    }))
}

fn ensure_known_hosts(config_dir: &Path) -> Result<PathBuf> {
    let path = config_dir.join("known_hosts");
    if !path.exists() {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("无法创建 known_hosts: {}", path.display()))?;
    }
    Ok(path)
}

fn write_temp_key(config_dir: &Path, key: &str) -> Result<TempPath> {
    let mut file = Builder::new()
        .prefix("ssh_key_")
        .tempfile_in(config_dir)
        .context("无法创建临时私钥文件")?;
    file.write_all(key.as_bytes())
        .context("写入临时私钥文件失败")?;
    file.flush().context("刷新临时私钥文件失败")?;

    let temp_path = file.into_temp_path();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perm = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&temp_path, perm).context("设置临时私钥权限失败")?;
    }

    Ok(temp_path)
}

fn quote_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('"', "\\\"");
    format!("\"{}\"", value)
}

fn try_add_key_to_agent(config_dir: &Path, key: &str) -> Result<bool> {
    if env::var("SSH_AUTH_SOCK").is_err() {
        return Ok(false);
    }

    let list_status = match Command::new("ssh-add").arg("-l").status() {
        Ok(status) => status,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("未找到 ssh-add，跳过 ssh-agent 集成。");
            return Ok(false);
        }
        Err(e) => return Err(anyhow::anyhow!("检查 ssh-agent 失败: {}", e)),
    };

    if let Some(code) = list_status.code()
        && code == 2
    {
        return Ok(false);
    }

    let temp_key = write_temp_key(config_dir, key)?;
    let add_status = Command::new("ssh-add")
        .arg(temp_key.as_ref() as &Path)
        .status()
        .context("执行 ssh-add 失败")?;

    if add_status.success() {
        return Ok(true);
    }

    eprintln!("ssh-add 未成功，继续使用临时私钥文件。");
    Ok(false)
}

