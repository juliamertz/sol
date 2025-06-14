mod analyzer;
mod ast;
mod codegen;
mod lexer;
mod parser;

#[cfg(test)]
mod tests;

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::{process, vec};
use std::str::FromStr;

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

mod spec {
    use std::str::FromStr;

    #[derive(Debug)]
    pub struct Raw {
        pub source_code: String,
        pub expected: String,
    }

    #[derive(Debug)]
    pub struct Spec<T> {
        pub source: T,
        pub expected: T,
    }

    impl<T: PartialEq + Eq + std::fmt::Debug> Spec<T> {
        fn eq(&self) -> bool {
            self.source == self.expected
        }
    }

    impl FromStr for Raw {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut lines = s.lines().filter(|line| !line.is_empty());

            let line = lines.next().unwrap();
            assert_eq!(line, "# Source");

            let line = lines.next().unwrap();
            assert!(line.starts_with("```"));
            let lang = line.strip_prefix("```").unwrap();
            assert_eq!(lang, "newlang");

            let mut source_code = String::new();
            let mut line = lines.next().unwrap();
            while !line.starts_with("```") {
                source_code.push_str(line);
                line = lines.next().unwrap();
            }

            let line = lines.next().unwrap();
            assert!(line.starts_with("# Expected"));

            let line = lines.next().unwrap();
            assert!(line.starts_with("```"));
            let lang = line.strip_prefix("```").unwrap();
            assert_eq!(lang, "ron");

            let mut expected = String::new();
            let mut line = lines.next().unwrap();
            while !line.starts_with("```") {
                expected.push_str(line);
                line = lines.next().unwrap();
            }

            Ok(Self {
                source_code,
                expected,
            })
        }
    }
}

fn main() -> Result<()> {
    // let text = std::fs::read_to_string("./src/tests/struct.spec.md").unwrap();
    // let raw_spec = spec::Raw::from_str(&text).unwrap();
    //
    // let parser = parser::Parser::new(raw_spec.source_code);
    // // let spec: spec::Spec<Vec<crate::ast::Node>> = spec::Spec {
    // //     source: parser.,
    // //     expected: vec![],
    // // };
    // dbg!(&raw_spec, &spec);
    //
    // std::process::exit(0);

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
