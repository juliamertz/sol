mod analyzer;
mod ast;
mod codegen;
mod lexer;
mod parser;

use std::{
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use clap::Parser;
use miette::{IntoDiagnostic, Result};

use crate::{
    analyzer::{Scope, TypeEnv, check_nodes},
    codegen::{Compiler, Emitter},
    lexer::TokenKind,
};

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
    DumpTokens {
        filepath: PathBuf,
    },
    DumpAst {
        filepath: PathBuf,
    },
}

fn build(filepath: &Path, opts: &BuildOpts) -> Result<PathBuf> {
    let content = std::fs::read_to_string(filepath).unwrap();

    let mut parser = parser::Parser::new(content);
    let ast = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            return Err(err);
        }
    };

    let mut env = TypeEnv::default();
    let mut scope = Scope::default();
    check_nodes(&ast, &mut env, &mut scope)?;

    let mut c = codegen::C::default();
    let out = c.emit(env, &ast);

    let outpath = c.build_exe(&out, "bin", opts)?;
    Ok(outpath)
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
            let _out = std::process::Command::new(&bin_path)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            if opts.cleanup {
                std::fs::remove_file(bin_path).into_diagnostic()?;
            }
        }
        Command::DumpTokens { filepath } => {
            let content = std::fs::read_to_string(filepath).unwrap();
            let mut stdout = std::io::stdout();
            let mut lex = lexer::Lexer::new(content);

            while let Some(token) = lex.read_token() {
                let kind = token.kind.to_string();
                stdout.write_all(kind.as_bytes()).unwrap();

                if !token.text.is_empty() && token.kind != TokenKind::Newline {
                    stdout.write_all(b" :: ").unwrap();
                    stdout.write_all(token.text.as_bytes()).unwrap();
                }

                stdout.write_all(b"\n").unwrap();
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
