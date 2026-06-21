use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};

use super::fallback;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStorage {
    Keyring,
    Fallback,
}

pub struct StoredKey {
    pub value: String,
    pub storage: KeyStorage,
}

fn entry() -> std::result::Result<Entry, KeyringError> {
    Entry::new(
        super::fallback::KEYCHAIN_SERVICE,
        super::fallback::KEYCHAIN_ACCOUNT,
    )
}

pub fn set_key(key: &str) -> Result<KeyStorage> {
    let entry = match entry() {
        Ok(value) => value,
        Err(e) => return fallback_on_platform_error(key, e, "创建钥匙串条目失败"),
    };

    match entry.set_password(key) {
        Ok(_) => {
            let _ = fallback::delete_fallback_key();
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
                return Ok(fallback::load_key()?.map(|value| StoredKey {
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
        Err(KeyringError::NoEntry) => Ok(fallback::load_key()?.map(|value| StoredKey {
            value,
            storage: KeyStorage::Fallback,
        })),
        Err(e) if is_platform_error(&e) => {
            warn_platform_fallback("读取系统钥匙串失败", &e);
            Ok(fallback::load_key()?.map(|value| StoredKey {
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

    fallback::delete_fallback_key().context("删除配置文件回退私钥失败")?;
    Ok(())
}

pub fn import_key_from_file(path: &std::path::Path) -> Result<KeyStorage> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取私钥文件: {}", path.display()))?;
    set_key(&content)
}

fn fallback_on_platform_error(key: &str, error: KeyringError, context: &str) -> Result<KeyStorage> {
    if is_platform_error(&error) {
        warn_platform_fallback(context, &error);
        fallback::store_key(key)?;
        Ok(KeyStorage::Fallback)
    } else {
        Err(anyhow::anyhow!("{}: {}", context, error))
    }
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

