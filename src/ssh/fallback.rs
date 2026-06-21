use anyhow::{Context, Result};
use std::env;

use crate::config::Config;

pub const KEYCHAIN_SERVICE: &str = "skillsync";
pub const KEYCHAIN_ACCOUNT: &str = "ssh_private_key";
const FALLBACK_PREFIX: &str = "xor-v1:";

pub fn store_key(key: &str) -> Result<()> {
    let mut config = Config::load_blocking()?;
    config.ssh_private_key_fallback = Some(encrypt(key));
    config.save_blocking()?;
    Ok(())
}

pub fn load_key() -> Result<Option<String>> {
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

    match decrypt(value) {
        Ok(value) => Ok(Some(value)),
        Err(e) => {
            eprintln!("回退私钥解析失败：{}。请重新导入私钥。", e);
            Ok(None)
        }
    }
}

pub fn delete_fallback_key() -> Result<()> {
    let mut config = Config::load_blocking()?;
    if config.ssh_private_key_fallback.is_some() {
        config.ssh_private_key_fallback = None;
        config.save_blocking()?;
    }
    Ok(())
}

fn encrypt(plain: &str) -> String {
    let key = derive_key();
    let mut out = Vec::with_capacity(plain.len());
    for (idx, b) in plain.as_bytes().iter().enumerate() {
        out.push(b ^ key[idx % key.len()]);
    }
    format!("{}{}", FALLBACK_PREFIX, hex_encode(&out))
}

fn decrypt(value: &str) -> Result<String> {
    if let Some(payload) = value.strip_prefix(FALLBACK_PREFIX) {
        let data = hex_decode(payload)?;
        let key = derive_key();
        let mut out = Vec::with_capacity(data.len());
        for (idx, b) in data.iter().enumerate() {
            out.push(b ^ key[idx % key.len()]);
        }
        return String::from_utf8(out).context("回退私钥解密失败");
    }
    Ok(value.to_string())
}

fn derive_key() -> [u8; 32] {
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

