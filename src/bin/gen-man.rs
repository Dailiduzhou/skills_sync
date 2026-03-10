use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Command, CommandFactory, Parser};
use clap_mangen::Man;

use skills_sync::cli::Cli;

#[derive(Parser)]
struct Args {
    /// 输出目录
    #[arg(long, default_value = "man")]
    out_dir: PathBuf,
}

fn write_man(out_dir: &Path, name: &str, cmd: &Command) -> Result<()> {
    let mut buffer = Vec::new();
    Man::new(cmd.clone()).render(&mut buffer)?;

    let file_name = format!("{}.1", name.replace(' ', "-"));
    let out_path = out_dir.join(file_name);
    std::fs::write(&out_path, buffer)?;

    println!("Wrote {}", out_path.display());
    Ok(())
}

fn render_tree(out_dir: &Path, cmd: &Command, prefix: &str) -> Result<()> {
    for sub in cmd.get_subcommands() {
        let mut sub_cmd = sub.clone();
        let full_name = format!("{} {}", prefix, sub_cmd.get_name());
        sub_cmd.set_bin_name(full_name.clone());

        write_man(out_dir, &full_name, &sub_cmd)?;
        render_tree(out_dir, &sub_cmd, &full_name)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out_dir)?;

    let mut root = Cli::command();
    let root_name = root.get_name().to_string();
    root.set_bin_name(root_name.clone());

    write_man(&args.out_dir, &root_name, &root)?;
    render_tree(&args.out_dir, &root, &root_name)?;
    Ok(())
}
