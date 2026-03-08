use anyhow::Result;

use crate::config::Config;

pub fn show(config: &Config) -> Result<()> {
    println!("{}", config.get_concurrency());
    Ok(())
}

pub async fn set(config: &mut Config, value: usize) -> Result<()> {
    config.set_concurrency(value).await?;
    println!("并发数已设置为 {}", value);
    Ok(())
}
