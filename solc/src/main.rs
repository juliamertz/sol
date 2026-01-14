mod type_checker;
mod codegen;
mod lexer;
mod parser;

use std::{
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use miette::{IntoDiagnostic, NamedSource, Result};

use crate::type_checker::{Scope, TypeEnv, check_nodes};
use crate::codegen::{Compiler, Emitter};

use lexer::SourceInfo;

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
        file_path: PathBuf,

        #[arg(short, long)]
        spans: bool,

        #[arg(short, long, default_value_t = 0)]
        take: usize,
    },
    DumpAst {
        file_path: PathBuf,
    },
}

fn build(file_path: &Path, opts: &BuildOpts) -> Result<PathBuf> {
    let content = std::fs::read_to_string(file_path).unwrap();
    let name = file_path.to_string_lossy();
    let source = SourceInfo::new(name, content.clone());

    let mut parser = parser::Parser::new(file_path.to_owned(), content);
    let ast = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            return Err(err.into());
        }
    };

    let mut env = TypeEnv::default();
    let mut scope = Scope::new(source);
    check_nodes(&ast, &mut env, &mut scope)?;

    let mut c = codegen::C::default();
    let out = c.emit(env, &ast);

    let outpath = c.build_exe(&out, "bin", opts)?;
    Ok(outpath)
}

fn main() -> Result<()> {
    let opts = Cli::parse();

    miette::set_hook(Box::new(|_| {
        let theme = miette::GraphicalTheme {
            characters: miette::ThemeCharacters::unicode(),
            styles: miette::ThemeStyles::rgb(),
        };
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .context_lines(3)
                .graphical_theme(theme)
                .build(),
        )
    }))
    .unwrap();

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
        Command::DumpTokens {
            file_path,
            spans,
            take,
        } => {
            let content = std::fs::read_to_string(&file_path).unwrap();
            let mut stdout = std::io::stdout();
            let mut lex = lexer::Lexer::new(file_path, content.clone());

            let mut tokens = vec![];
            let mut idx = 0;
            while let Some(token) = lex.read_token() {
                tokens.push(token);
                idx += 1;
                if idx > 0 && idx >= take {
                    break;
                }
            }

            if spans {
                let src = Arc::new(NamedSource::new("source", content));
                for token in tokens {
                    let report = miette::miette!(
                        labels = vec![miette::LabeledSpan::at(
                            token.span.offset()..token.span.offset() + token.span.len(),
                            format!("{:?}", token.kind)
                        )],
                        "{:?}",
                        token.kind
                    )
                    .with_source_code(src.clone());
                    println!("{:?}\n", report);
                }
            } else {
                for token in tokens {
                    let kind = token.kind.to_string();
                    stdout.write_all(kind.as_bytes()).unwrap();

                    if !token.text.is_empty() && token.kind != lexer::TokenKind::Newline {
                        stdout.write_all(b" :: ").unwrap();
                        stdout.write_all(token.text.as_bytes()).unwrap();
                    }

                    stdout.write_all(b"\n").unwrap();
                }
            }
        }
        Command::DumpAst { file_path } => {
            let content = std::fs::read_to_string(&file_path).unwrap();
            let mut parser = parser::Parser::new(file_path, content);
            let ast = parser.parse()?;
            dbg!(ast);
        }
    }

    Ok(())
}
