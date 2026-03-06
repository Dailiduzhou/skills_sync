use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::{Builder, TempPath};

const KEYCHAIN_SERVICE: &str = "skillsync";
const KEYCHAIN_ACCOUNT: &str = "ssh_private_key";

pub struct PreparedKey {
    pub ssh_command: String,
    pub _temp_key: TempPath,
}

fn entry() -> Result<Entry> {
    Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).context("创建钥匙串条目失败")
}

pub fn set_key(key: &str) -> Result<()> {
    entry()?.set_password(key).context("写入系统钥匙串失败")?;
    Ok(())
}

pub fn get_key() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(e).context("读取系统钥匙串失败"),
    }
}

pub fn delete_key() -> Result<()> {
    match entry()?.delete_password() {
        Ok(_) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(e).context("删除系统钥匙串内容失败"),
    }
}

pub fn import_key_from_file(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("无法读取私钥文件: {}", path.display()))?;
    set_key(&content)
}

pub fn prepare_git_ssh_command(config_dir: &Path) -> Result<Option<PreparedKey>> {
    let key = match get_key()? {
        Some(value) => value,
        None => return Ok(None),
    };

    let known_hosts = ensure_known_hosts(config_dir)?;
    let temp_key = write_temp_key(config_dir, &key)?;

    let key_path = quote_path(temp_key.as_ref());
    let known_hosts_path = quote_path(&known_hosts);

    let ssh_command = format!(
        "ssh -i {} -o IdentitiesOnly=yes -o UserKnownHostsFile={} -o StrictHostKeyChecking=accept-new",
        key_path, known_hosts_path
    );

    Ok(Some(PreparedKey {
        ssh_command,
        _temp_key: temp_key,
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
