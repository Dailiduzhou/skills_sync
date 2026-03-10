use std::{env, fs, path::PathBuf};

use anyhow::{bail, Result};
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// 输出目录
    #[arg(long, default_value = "man")]
    out_dir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let source_dir = PathBuf::from(env!("OUT_DIR")).join("man");
    if !source_dir.is_dir() {
        bail!("man pages not found at {}", source_dir.display());
    }

    fs::create_dir_all(&args.out_dir)?;
    for entry in fs::read_dir(&source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let dest = args.out_dir.join(entry.file_name());
        fs::copy(&path, &dest)?;
        println!("Wrote {}", dest.display());
    }
    Ok(())
}
