mod ast;
mod codegen;
mod lexer;
mod loc;
mod parser;

#[cfg(test)]
mod tests;

use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process,
};

use clap::Parser;
use codegen::{Compiler, Emitter, ReleaseType};
use miette::{IntoDiagnostic, Result};
use ron::ser::PrettyConfig;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct BuildOpts {
    #[arg(short, long, default_value = "debug")]
    release: ReleaseType,

    #[arg(short, long, default_value = "out")]
    outdir: PathBuf,

    #[arg(short, long)]
    cleanup: bool,
}

#[derive(clap::Subcommand)]
enum Command {
    Build {
        filepath: PathBuf,

        #[clap(flatten)]
        opts: BuildOpts,
    },
    Run {
        filepath: PathBuf,

        #[clap(flatten)]
        opts: BuildOpts,
    },
}

fn build(filepath: &Path, opts: &BuildOpts) -> Result<PathBuf> {
    let content = std::fs::read_to_string(&filepath).unwrap();

    let mut parser = parser::Parser::new(content);
    let nodes = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            return Err(err);
        }
    };

    println!("{}", ron::ser::to_string_pretty(&nodes, PrettyConfig::new()).unwrap());

    let mut emitter = codegen::C::default();
    let out = emitter.emit(&nodes);
    Ok(emitter.build_exe(&out, "test", &opts))
}

fn main() -> Result<()> {
    let opts = Cli::parse();

    match opts.command {
        Command::Build { filepath, opts } => {
            let bin_path = build(&filepath, &opts)?;
            let metadata = std::fs::metadata(&bin_path).into_diagnostic()?;
            println!("{} bytes written to {bin_path:?}", metadata.size());
        }
        Command::Run { filepath, opts } => {
            let bin_path = build(&filepath, &opts)?;
            let out = process::Command::new(&bin_path)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            if opts.cleanup {
                std::fs::remove_file(bin_path).into_diagnostic()?;
            }
        }
    }

    Ok(())
}
