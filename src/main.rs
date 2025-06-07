mod loc;
mod ast;
mod codegen;
mod lexer;
mod parser;

#[cfg(test)]
mod tests;

use std::{path::PathBuf, process};

use clap::Parser;
use codegen::{Compiler, Emitter};
use miette::Result;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Run {
        filepath: PathBuf,
    },
}

fn main() -> Result<()> {
    let opts = Cli::parse();

    match opts.command {
        Command::Run { filepath } => {
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
            let bin_path = emitter.build_exe(
                &out,
                "test",
                codegen::CCOpts {
                    cleanup: false,
                    release: codegen::ReleaseType::Fast,
                },
            );

            let out = process::Command::new(bin_path)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
        }
    }

    Ok(())
}
