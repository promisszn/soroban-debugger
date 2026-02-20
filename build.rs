use clap::CommandFactory;
use std::fs;
use std::path::Path;

// Mock crate root modules that src/cli/args.rs depends on
#[allow(dead_code)]
mod config {
    pub struct Config {
        pub debug: DebugConfig,
        pub output: OutputConfig,
    }
    pub struct DebugConfig {
        pub breakpoints: Vec<String>,
        pub verbosity: Option<u8>,
    }
    pub struct OutputConfig {
        pub format: Option<String>,
        pub show_events: Option<bool>,
    }
}

#[allow(dead_code)]
#[path = "src/cli/args.rs"]
mod args;

use args::Cli;

fn main() -> std::io::Result<()> {
    // Generate man page in the man/man1 directory
    let man_dir = Path::new("man").join("man1");
    fs::create_dir_all(&man_dir)?;

    let cmd = Cli::command();
    render_recursive(&cmd, &man_dir, "")?;

    println!("cargo:rerun-if-changed=src/cli/args.rs");
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

fn render_recursive(cmd: &clap::Command, out_dir: &Path, prefix: &str) -> std::io::Result<()> {
    let name = if prefix.is_empty() {
        cmd.get_name().to_string()
    } else {
        format!("{}-{}", prefix, cmd.get_name())
    };

    let cmd = cmd.clone().name(name.clone());
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    fs::write(out_dir.join(format!("{}.1", name)), buffer)?;

    for sub in cmd.get_subcommands() {
        if !sub.is_hide_set() {
            render_recursive(sub, out_dir, &name)?;
        }
    }

    Ok(())
}
