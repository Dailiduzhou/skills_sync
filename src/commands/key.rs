use anyhow::Result;
use std::path::PathBuf;

use crate::cli::KeyCommands;
use crate::ssh_key;

pub fn run(command: KeyCommands) -> Result<()> {
    match command {
        KeyCommands::Import { path } => {
            let path: PathBuf = PathBuf::from(path);
            let storage = ssh_key::import_key_from_file(&path)?;
            match storage {
                ssh_key::KeyStorage::Keyring => {
                    println!("已将私钥保存到系统钥匙串。");
                }
                ssh_key::KeyStorage::Fallback => {
                    println!("系统钥匙串不可用，已使用配置文件回退保存（弱加密）。");
                }
            }
        }
        KeyCommands::Remove => {
            ssh_key::delete_key()?;
            println!("已删除已保存的 SSH 私钥（如有）。");
        }
        KeyCommands::Status => {
            match ssh_key::get_key()? {
                Some(stored) => match stored.storage {
                    ssh_key::KeyStorage::Keyring => {
                        println!("已保存 SSH 私钥（系统钥匙串）。");
                    }
                    ssh_key::KeyStorage::Fallback => {
                        println!("已保存 SSH 私钥（配置文件回退）。");
                    }
                },
                None => {
                    println!("未保存 SSH 私钥。");
                }
            }
        }
    }
    Ok(())
}
