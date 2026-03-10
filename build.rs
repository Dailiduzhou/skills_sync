use clap::{Command, CommandFactory};
use clap_complete::{generate_to, shells};
use clap_mangen::Man;
use std::{
    env,
    fs,
    io,
    path::{Path, PathBuf},
};

mod cli {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cli.rs"));
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src/cli.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let mut cmd = cli::Cli::command();

    let _ = generate_to(shells::Bash, &mut cmd, "skillsync", &out_dir)?;
    let _ = generate_to(shells::Zsh, &mut cmd, "skillsync", &out_dir)?;
    let _ = generate_to(shells::Fish, &mut cmd, "skillsync", &out_dir)?;
    let _ = generate_to(shells::PowerShell, &mut cmd, "skillsync", &out_dir)?;
    let _ = generate_to(shells::Elvish, &mut cmd, "skillsync", &out_dir)?;

    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir)?;

    let mut root = cli::Cli::command();
    let root_name = root.get_name().to_string();
    root.set_bin_name(root_name.clone());

    write_man(&man_dir, &root_name, &root)?;
    render_tree(&man_dir, &root, &root_name)?;

    Ok(())
}

fn write_man(out_dir: &Path, name: &str, cmd: &Command) -> io::Result<()> {
    let mut buffer = Vec::new();
    Man::new(cmd.clone())
        .render(&mut buffer)
        .map_err(io::Error::other)?;

    let file_name = format!("{}.1", name.replace(' ', "-"));
    let out_path = out_dir.join(file_name);
    fs::write(&out_path, buffer)?;
    Ok(())
}

fn render_tree(out_dir: &Path, cmd: &Command, prefix: &str) -> io::Result<()> {
    for sub in cmd.get_subcommands() {
        let mut sub_cmd = sub.clone();
        let full_name = format!("{} {}", prefix, sub_cmd.get_name());
        sub_cmd.set_bin_name(full_name.clone());

        write_man(out_dir, &full_name, &sub_cmd)?;
        render_tree(out_dir, &sub_cmd, &full_name)?;
    }
    Ok(())
}
