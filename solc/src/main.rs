use std::{
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use clap::Parser;
use miette::{IntoDiagnostic, Result};

use solc::{
    codegen::{self, qbe},
    hir, lexer, mir, parser,
    type_checker::{self, Scope, TypeEnv},
};

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
struct BuildOpts {
    /// Path to directory which all build artifacts get written to
    #[arg(short, long, default_value = "out")]
    outdir: PathBuf,

    /// Whether to clean up build artifacts
    #[arg(short, long)]
    cleanup: bool,
}

#[derive(clap::Subcommand)]
enum DumpCommand {
    Tokens {
        #[arg(short, long)]
        spans: bool,

        #[arg(short, long, default_value_t = 0)]
        take: usize,
    },
    Ast,
    Hir,
    Mir,
    Qbe,
}

#[derive(clap::Subcommand)]
enum Command {
    Build {
        file_path: PathBuf,

        #[clap(flatten)]
        opts: BuildOpts,
    },
    Run {
        file_path: PathBuf,

        #[clap(flatten)]
        opts: BuildOpts,
    },
    Dump {
        file_path: PathBuf,

        #[command(subcommand)]
        cmd: DumpCommand,
    },
}

fn init_miette() {
    miette::set_hook(Box::new(|_| {
        let theme = miette::GraphicalTheme {
            characters: miette::ThemeCharacters::unicode(),
            styles: miette::ThemeStyles::ansi(),
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
}

fn build(file_path: &Path, opts: &BuildOpts) -> Result<PathBuf> {
    let content = std::fs::read_to_string(file_path).unwrap();
    let _name = file_path.to_string_lossy();

    let mut parser = parser::Parser::new(file_path.to_owned(), &content)?;
    let module_ast = parser.parse()?;

    let mut env = TypeEnv::new(parser.source());
    let mut scope = Scope::default();

    type_checker::check_module(&module_ast, &mut env, &mut scope)?;

    let module_hir = hir::lower_module(&module_ast, &mut env)?;
    let module_mir = mir::lower_module(&module_hir, &env)?;

    let mut qbe_builder = codegen::qbe::lower::Builder::new(&env);
    let qbe_module = qbe_builder.lower_module(&module_mir)?;

    let compiler = codegen::qbe::compile::Compiler::new(&opts.outdir);
    let out_path = compiler.ir_to_bin(&qbe_module)?;

    Ok(out_path)
}

fn write_str(mut w: impl Write, str: impl ToString) -> Result<(), miette::ErrReport> {
    w.write_all(str.to_string().as_bytes()).into_diagnostic()
}

fn main() -> Result<()> {
    let opts = Cli::parse();
    init_miette();

    match opts.command {
        Command::Build { file_path, opts } => {
            let bin_path = build(&file_path, &opts)?;
            let metadata = std::fs::metadata(&bin_path).into_diagnostic()?;
            println!("{} bytes written to {bin_path:?}", metadata.size());
        }

        Command::Run { file_path, opts } => {
            let bin_path = build(&file_path, &opts)?;
            let _out = std::process::Command::new(&bin_path)
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            if opts.cleanup {
                std::fs::remove_file(bin_path).into_diagnostic()?;
            }
        }

        Command::Dump { file_path, cmd } => {
            let content = std::fs::read_to_string(&file_path).unwrap();
            let stdout = std::io::stdout();

            if let DumpCommand::Tokens { spans, take } = cmd {
                return dump_tokens(file_path, spans, take);
            }

            let mut parser = parser::Parser::new(file_path, &content)?;
            let ast = parser.parse()?;
            if let DumpCommand::Ast = cmd {
                let fmt = solc::ast::fmt::FmtModule::new(&ast, &content).to_string();
                return write_str(stdout, fmt);
            }

            let mut env = TypeEnv::new(parser.source());
            let mut scope = Scope::default();
            type_checker::check_module(&ast, &mut env, &mut scope)?;

            let hir = hir::lower_module(&ast, &mut env)?;
            if let DumpCommand::Hir = cmd {
                return write_str(stdout, format!("{hir:#?}"));
            }

            let mir = mir::lower_module(&hir, &env)?;
            if let DumpCommand::Mir = cmd {
                return write_str(stdout, mir);
            }

            let mut qbe = qbe::lower::Builder::new(&env);
            let ssa = qbe.lower_module(&mir)?;
            if let DumpCommand::Qbe = cmd {
                return write_str(stdout, ssa);
            }

            unreachable!()
        }
    }

    Ok(())
}

fn dump_tokens(file_path: PathBuf, spans: bool, take: usize) -> Result<()> {
    let content = std::fs::read_to_string(&file_path).unwrap();
    let mut stdout = std::io::stdout();
    let mut lex = lexer::Lexer::new(file_path, &content);

    let mut tokens = vec![];
    let mut idx = 0;
    while let Some(result) = lex.read_token() {
        match result {
            Ok(token) => {
                tokens.push(token);
                idx += 1;
                if idx > 0 && idx >= take {
                    break;
                }
            }
            Err(err) => {
                eprintln!("error reading token: {err:?}");
                break;
            }
        }
    }

    if spans {
        let src = lex.source();
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

    Ok(())
}
