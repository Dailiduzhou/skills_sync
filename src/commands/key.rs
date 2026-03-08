use anyhow::Result;
use std::path::PathBuf;

use crate::cli::KeyCommands;
use crate::ssh_key;

pub fn run(command: KeyCommands) -> Result<()> {
    match command {
        KeyCommands::Import { path } => {
            let path: PathBuf = PathBuf::from(path);
            ssh_key::import_key_from_file(&path)?;
            println!("已将私钥保存到系统钥匙串。");
        }
        KeyCommands::Remove => {
            ssh_key::delete_key()?;
            println!("已从系统钥匙串删除私钥。");
        }
        KeyCommands::Status => {
            if ssh_key::get_key()?.is_some() {
                println!("已保存 SSH 私钥。");
            } else {
                println!("未保存 SSH 私钥。");
            }
        }
    }
    Ok(())
}
