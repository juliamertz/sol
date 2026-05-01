use std::io::Write;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};

use lib::codegen::qbe;
use lib::lexer::Token;
use lib::lexer::source::SourceInfo;
use lib::{ast, hir, lexer, mir, parser, type_checker};

#[derive(clap::Args)]
pub struct DumpOpts {
    file_path: PathBuf,

    #[command(subcommand)]
    cmd: DumpCommand,
}

#[derive(clap::Subcommand)]
pub enum DumpCommand {
    Tokens {
        #[arg(short, long)]
        spans: bool,

        #[arg(short, long)]
        take: Option<usize>,
    },
    Ast,
    Hir,
    Mir,
    Qbe,
}

fn write_str(mut w: impl Write, str: impl ToString) -> Result<(), miette::ErrReport> {
    w.write_all(str.to_string().as_bytes()).into_diagnostic()
}

fn print_token_span(w: &mut impl Write, token: Token<'_>, src: SourceInfo) {
    let report = miette::miette!(
        labels = vec![miette::LabeledSpan::at(
            token.span.offset()..token.span.offset() + token.span.len(),
            format!("{:?}", token.kind)
        )],
        "{:?}",
        token.kind
    )
    .with_source_code(src);
    writeln!(w, "{:?}", report).unwrap();
}

fn print_token(w: &mut impl Write, token: Token<'_>) {
    let kind = token.kind.to_string();
    w.write_all(kind.as_bytes()).unwrap();

    if !token.text.is_empty() && token.kind != lexer::TokenKind::Newline {
        w.write_all(b" :: ").unwrap();
        w.write_all(token.text.as_bytes()).unwrap();
    }

    w.write_all(b"\n").unwrap();
}

fn dump_tokens(file_path: PathBuf, spans: bool, take: Option<usize>) -> Result<()> {
    let content = std::fs::read_to_string(&file_path).unwrap();
    let lex = lexer::Lexer::new(file_path, &content);
    let src = lex.source();

    let mut stdout = std::io::stdout();
    let iter = lex.take(take.unwrap_or(usize::MAX));

    for result in iter {
        match result {
            Ok(token) => {
                if spans {
                    print_token_span(&mut stdout, token, src.clone());
                } else {
                    print_token(&mut stdout, token);
                }
            }
            Err(err) => eprintln!("failed to read token: {err:#}"),
        }
    }

    Ok(())
}

pub fn handle(DumpOpts { file_path, cmd }: DumpOpts) -> Result<()> {
    let content = std::fs::read_to_string(&file_path).unwrap();
    let stdout = std::io::stdout();

    if let DumpCommand::Tokens { spans, take } = cmd {
        return dump_tokens(file_path, spans, take);
    }

    let mut parser = parser::Parser::new(file_path, &content)?;
    let ast = parser.parse()?;
    if let DumpCommand::Ast = cmd {
        let fmt = ast::fmt::FmtModule::new(&ast, &content).to_string();
        return write_str(stdout, fmt);
    }

    let mut env = type_checker::TypeEnv::new(parser.source());
    let mut scope = type_checker::Scope::default();
    type_checker::check_module(&ast, &mut env, &mut scope)?;

    let hir = hir::lower_module(&ast, &mut env)?;
    if let DumpCommand::Hir = cmd {
        return write_str(stdout, format!("{hir:#?}"));
    }

    let mir = mir::lower_module(&hir, &env)?;
    if let DumpCommand::Mir = cmd {
        let printer = mir::fmt::MirPrinter::new(mir, &env);
        return write_str(stdout, printer);
    }

    let mut qbe = qbe::lower::Builder::new(&env);
    let ssa = qbe.lower_module(&mir)?;
    if let DumpCommand::Qbe = cmd {
        return write_str(stdout, ssa);
    }

    unreachable!()
}
