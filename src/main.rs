mod analyzer;
mod ast;
mod codegen;
mod hir;
mod lexer;
mod parser;

#[cfg(test)]
mod tests;

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process;

use analyzer::TypeEnv;
use clap::Parser;
use codegen::{Compiler, Emitter};
use miette::{IntoDiagnostic, Result};

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct BuildOpts {
    /// Set release mode enabling all optimizations
    #[arg(short, long)]
    release: bool,

    /// Path to directory which all build artifacts get written to
    #[arg(short, long, default_value = "out")]
    outdir: PathBuf,

    /// Whether to clean up build artifacts
    #[arg(short, long, default_value_t = !cfg!(debug_assertions))]
    cleanup: bool,
}

#[derive(clap::Subcommand)]
enum Command {
    Build {
        filepath: PathBuf,

        // TODO: implement
        #[arg(short, long)]
        skip_codegen: bool,

        #[clap(flatten)]
        opts: BuildOpts,
    },
    Run {
        filepath: PathBuf,

        #[clap(flatten)]
        opts: BuildOpts,
    },
    DumpAst {
        filepath: PathBuf,
    },
}

fn build(filepath: &Path, opts: &BuildOpts) -> Result<PathBuf> {
    let content = std::fs::read_to_string(filepath).unwrap();

    let mut parser = parser::Parser::new(content);
    let nodes = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            return Err(err);
        }
    };

    let mut builder = crate::hir::HirBuilder::default();
    let mut env = crate::hir::TypeEnv::default();
    let hir = builder.lower(nodes, &mut env)?;

    // let mut emitter = codegen::C::default();
    // let mut env = TypeEnv::new();

    // let out = codegen::Js.emit(&hir);
    // let outpath = PathBuf::from("./out/source.js") ;
    let out = codegen::C.emit(&hir);
    let outpath = PathBuf::from("./out/source.c") ;
    std::fs::write(&outpath, out).unwrap();
    Ok(outpath)
}

fn main() -> Result<()> {
    let opts = Cli::parse();

    match opts.command {
        Command::Build {
            filepath,
            skip_codegen,
            opts,
        } => {
            let bin_path = build(&filepath, &opts)?;
            let metadata = std::fs::metadata(&bin_path).into_diagnostic()?;
            println!("{} bytes written to {bin_path:?}", metadata.size());
        }

        Command::Run { filepath, opts } => {
            let bin_path = build(&filepath, &opts)?;
            let _out = process::Command::new(&bin_path)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            if opts.cleanup {
                std::fs::remove_file(bin_path).into_diagnostic()?;
            }
        }
        Command::DumpAst { filepath } => {
            let content = std::fs::read_to_string(filepath).unwrap();
            let mut parser = parser::Parser::new(content);
            let ast = parser.parse()?;
            dbg!(ast);
        }
    }

    Ok(())
}
