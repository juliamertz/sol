mod analyzer;
mod ast;
mod codegen;
mod lexer;
mod parser;

#[cfg(test)]
mod tests;

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process;

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
    let content = std::fs::read_to_string(filepath).unwrap();

    let mut parser = parser::Parser::new(content);
    let nodes = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            return Err(err);
        }
    };

    let mut emitter = codegen::C::default();
    let out = emitter.emit(&nodes);
    emitter.build_exe(&out, "test", opts)
}

// struct Spec<'a> {
//     source_code: &'a str,
//     expected: &'a str,
// }

fn parse_spec() {
    use comrak::nodes::{AstNode, NodeValue};
    use comrak::{Arena, Options, format_html, parse_document};

    let text = std::fs::read_to_string("./src/tests/struct.spec.md").unwrap();
    let arena = Arena::new();
    let opts = Options::default();
    let root = comrak::parse_document(&arena, &text, &opts);

    let mut children = root.descendants();
    children.next();

    match children.next().unwrap().data.borrow().value {
        NodeValue::Text(ref title) => {
            dbg!(title);
        },
        // NodeValue::CodeBlock(ref mut codeblock) => {
        //     dbg!(codeblock);
        // },
        ref x => todo!("{x:?}"),
    }

    // let NodeValue::Text(title) =  children.next().unwrap().data.borrow() else {
    //     panic!();
    // };

    // for node in root.descendants() {
    //     match node.data.borrow_mut().value {
    //         NodeValue::CodeBlock(ref mut codeblock) => {
    //             dbg!(codeblock);
    //         }
    //
    //         NodeValue::CodeBlock(ref mut codeblock) => {
    //             dbg!(codeblock);
    //         }
    //
    //         ref val => {
    //             dbg!(val);
    //         }
    //     }
    //     // dbg!(&node.data);
    // }
}

fn main() -> Result<()> {
    parse_spec();
    std::process::exit(0);

    let opts = Cli::parse();

    match opts.command {
        Command::Build { filepath, opts } => {
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
    }

    Ok(())
}
