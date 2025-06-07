mod ast;
mod codegen;
mod lexer;
mod loc;
mod parser;

#[cfg(test)]
mod tests;

use std::{
    path::{Path, PathBuf},
    process,
};

use clap::Parser;
use codegen::{Compiler, Emitter, ReleaseType};
use miette::Result;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct BuildOpts {
    #[arg(short, long)]
    release: ReleaseType,
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

fn build(filepath: &Path, opts: BuildOpts) -> Result<PathBuf> {
    let content = std::fs::read_to_string(&filepath).unwrap();

    let mut parser = parser::Parser::new(content);
    let nodes = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            // dbg!(&parser.tokens);
            return Err(err);
        }
    };

    dbg!(&nodes);

    let mut emitter = codegen::C::default();
    let out = emitter.emit(&nodes);
    Ok(emitter.build_exe(
        &out,
        "test",
        codegen::CCOpts {
            cleanup: false,
            release: codegen::ReleaseType::Fast,
        },
    ))
}

fn main() -> Result<()> {
    let opts = Cli::parse();

    match opts.command {
        Command::Build { filepath, opts } => {
            let bin_path = build(&filepath, opts)?;
            dbg!(bin_path);
        }
        Command::Run { filepath, opts } => {
            let bin_path = build(&filepath, opts)?;
            let out = process::Command::new(bin_path)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
        }
    }

    Ok(())
}
