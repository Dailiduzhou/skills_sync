use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{Builder, TempPath};

use crate::config::Config;

const KEYCHAIN_SERVICE: &str = "skillsync";
const KEYCHAIN_ACCOUNT: &str = "ssh_private_key";
const FALLBACK_PREFIX: &str = "xor-v1:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStorage {
    Keyring,
    Fallback,
}

pub struct StoredKey {
    pub value: String,
    pub storage: KeyStorage,
}

pub struct PreparedKey {
    pub ssh_command: String,
    pub _temp_key: Option<TempPath>,
}

fn entry() -> std::result::Result<Entry, KeyringError> {
    Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
}

pub fn set_key(key: &str) -> Result<KeyStorage> {
    let entry = match entry() {
        Ok(value) => value,
        Err(e) => return fallback_on_platform_error(key, e, "创建钥匙串条目失败"),
    };

    match entry.set_password(key) {
        Ok(_) => {
            let _ = delete_fallback_key();
            Ok(KeyStorage::Keyring)
        }
        Err(e) => fallback_on_platform_error(key, e, "写入系统钥匙串失败"),
    }
}

pub fn get_key() -> Result<Option<StoredKey>> {
    let entry = match entry() {
        Ok(value) => value,
        Err(e) => {
            if is_platform_error(&e) {
                warn_platform_fallback("无法访问系统钥匙串", &e);
                return Ok(load_fallback_key()?.map(|value| StoredKey {
                    value,
                    storage: KeyStorage::Fallback,
                }));
            }
            return Err(anyhow::anyhow!("创建钥匙串条目失败: {}", e));
        }
    };

    match entry.get_password() {
        Ok(value) => Ok(Some(StoredKey {
            value,
            storage: KeyStorage::Keyring,
        })),
        Err(KeyringError::NoEntry) => Ok(load_fallback_key()?.map(|value| StoredKey {
            value,
            storage: KeyStorage::Fallback,
        })),
        Err(e) if is_platform_error(&e) => {
            warn_platform_fallback("读取系统钥匙串失败", &e);
            Ok(load_fallback_key()?.map(|value| StoredKey {
                value,
                storage: KeyStorage::Fallback,
            }))
        }
        Err(e) => Err(anyhow::anyhow!("读取系统钥匙串失败: {}", e)),
    }
}

pub fn delete_key() -> Result<()> {
    match entry() {
        Ok(entry) => match entry.delete_password() {
            Ok(_) | Err(KeyringError::NoEntry) => {}
            Err(e) if is_platform_error(&e) => {
                warn_platform_fallback("删除系统钥匙串失败", &e);
            }
            Err(e) => return Err(anyhow::anyhow!("删除系统钥匙串内容失败: {}", e)),
        },
        Err(e) => {
            if is_platform_error(&e) {
                warn_platform_fallback("无法访问系统钥匙串", &e);
            } else {
                return Err(anyhow::anyhow!("创建钥匙串条目失败: {}", e));
            }
        }
    }

    delete_fallback_key().context("删除配置文件回退私钥失败")?;
    Ok(())
}

pub fn import_key_from_file(path: &Path) -> Result<KeyStorage> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("无法读取私钥文件: {}", path.display()))?;
    set_key(&content)
}

pub fn prepare_git_ssh_command(config_dir: &Path) -> Result<Option<PreparedKey>> {
    let stored_key = match get_key()? {
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

fn fallback_on_platform_error(key: &str, error: KeyringError, context: &str) -> Result<KeyStorage> {
    if is_platform_error(&error) {
        warn_platform_fallback(context, &error);
        store_fallback_key(key)?;
        return Ok(KeyStorage::Fallback);
    }
    Err(anyhow::anyhow!("{}: {}", context, error))
}

fn is_platform_error(error: &KeyringError) -> bool {
    matches!(
        error,
        KeyringError::PlatformFailure(_) | KeyringError::NoStorageAccess(_)
    )
}

fn warn_platform_fallback(context: &str, error: &KeyringError) {
    let detail = error.to_string();
    if detail.contains("ServiceUnknown") {
        eprintln!(
            "{}：当前环境未运行凭据管理服务 (Secret Service)。已使用配置文件回退存储（弱加密）。",
            context
        );
    } else if matches!(error, KeyringError::NoStorageAccess(_)) {
        eprintln!(
            "{}：无法访问系统凭据存储（可能被锁定或权限不足）。已使用配置文件回退存储（弱加密）。",
            context
        );
    } else {
        eprintln!(
            "{}：系统凭据存储不可用 ({}). 已使用配置文件回退存储（弱加密）。",
            context, detail
        );
    }
}

fn store_fallback_key(key: &str) -> Result<()> {
    let mut config = Config::load_blocking()?;
    config.ssh_private_key_fallback = Some(encrypt_fallback(key));
    config.save_blocking()?;
    Ok(())
}

fn load_fallback_key() -> Result<Option<String>> {
    let config = match Config::load_blocking() {
        Ok(value) => value,
        Err(e) => {
            eprintln!("无法读取配置文件以加载回退私钥：{}", e);
            return Ok(None);
        }
    };

    let value = match config.ssh_private_key_fallback.as_deref() {
        Some(value) => value,
        None => return Ok(None),
    };

    match decrypt_fallback(value) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            eprintln!("回退私钥解析失败：{}。请重新导入私钥。", e);
            Ok(None)
        }
    }
}

fn delete_fallback_key() -> Result<()> {
    let mut config = Config::load_blocking()?;
    if config.ssh_private_key_fallback.is_some() {
        config.ssh_private_key_fallback = None;
        config.save_blocking()?;
    }
    Ok(())
}

fn encrypt_fallback(plain: &str) -> String {
    let key = derive_fallback_key();
    let mut out = Vec::with_capacity(plain.len());
    for (idx, b) in plain.as_bytes().iter().enumerate() {
        out.push(b ^ key[idx % key.len()]);
    }
    format!("{}{}", FALLBACK_PREFIX, hex_encode(&out))
}

fn decrypt_fallback(value: &str) -> Result<String> {
    if let Some(payload) = value.strip_prefix(FALLBACK_PREFIX) {
        let data = hex_decode(payload)?;
        let key = derive_fallback_key();
        let mut out = Vec::with_capacity(data.len());
        for (idx, b) in data.iter().enumerate() {
            out.push(b ^ key[idx % key.len()]);
        }
        return String::from_utf8(out).context("回退私钥解密失败");
    }
    Ok(value.to_string())
}

fn derive_fallback_key() -> [u8; 32] {
    let mut seed = Vec::new();
    seed.extend_from_slice(b"skillsync-fallback-v1");
    if let Ok(user) = env::var("USER") {
        seed.extend_from_slice(user.as_bytes());
    }
    if let Ok(home) = env::var("HOME") {
        seed.extend_from_slice(home.as_bytes());
    }
    if let Ok(host) = env::var("HOSTNAME") {
        seed.extend_from_slice(host.as_bytes());
    }

    let mut state = fnv1a64(&seed);
    let mut key = [0u8; 32];
    for slot in key.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *slot = (state >> 56) as u8 ^ (state >> 32) as u8 ^ (state >> 8) as u8;
    }
    key
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(value: &str) -> Result<Vec<u8>> {
    let bytes = value.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        anyhow::bail!("hex 长度非法");
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut idx = 0;
    while idx < bytes.len() {
        let hi = hex_val(bytes[idx])?;
        let lo = hex_val(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
        idx += 2;
    }
    Ok(out)
}

fn hex_val(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => anyhow::bail!("hex 字符非法"),
    }
}
