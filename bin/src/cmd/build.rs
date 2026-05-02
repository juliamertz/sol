use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::{fs, time};

use miette::{IntoDiagnostic, Result};

use lib::codegen::qbe;
use lib::{hir, mir, parser, type_checker};
use tracing::instrument;

#[derive(clap::Args)]
pub struct BuildOpts {
    /// Path to source code
    pub file_path: PathBuf,

    /// Path to directory which all build artifacts get written to
    #[arg(short, long, default_value = "out")]
    pub out_dir: PathBuf,

    /// Whether to clean up build artifacts
    #[arg(short, long)]
    pub cleanup: bool,
}

#[instrument(skip_all, fields(file_path = ?opts.file_path, out_dir = ?opts.out_dir ), err(Debug))]
pub fn build(opts: &BuildOpts) -> Result<PathBuf> {
    let file_path = opts.file_path.as_path();
    let content = std::fs::read_to_string(file_path).into_diagnostic()?;
    let _name = file_path.to_string_lossy();
    tracing::debug!("read {} bytes from source file", content.len());

    tracing::debug!("starting build");

    let start = time::Instant::now();
    let now = start;
    let mut parser = parser::Parser::new(file_path.to_owned(), &content)?;
    let module_ast = parser.module()?;
    tracing::debug!({ elapsed = ?now.elapsed() }, "done parsing");

    let now = time::Instant::now();
    let mut env = type_checker::TypeEnv::new(parser.source());
    let mut scope = type_checker::Scope::default();
    type_checker::check_module(&module_ast, &mut env, &mut scope)?;
    tracing::debug!({ elapsed = ?now.elapsed() }, "done type checking");

    let now = time::Instant::now();
    let module_hir = hir::lower_module(&module_ast, &mut env)?;
    tracing::debug!({ elapsed = ?now.elapsed() }, "done lowering to HIR");

    let now = time::Instant::now();
    let module_mir = mir::lower_module(&module_hir, &env)?;
    tracing::debug!({ elapsed = ?now.elapsed() }, "done lowering to MIR");

    let now = time::Instant::now();
    let mut qbe_builder = qbe::lower::Builder::new(&env);
    let qbe_module = qbe_builder.lower_module(&module_mir)?;
    tracing::debug!({ elapsed = ?now.elapsed() }, "done generating QBE IR");

    let _ = fs::create_dir(&opts.out_dir);
    let compiler = qbe::compile::Compiler::new(&opts.out_dir);
    let out_path = compiler.ir_to_bin(&qbe_module)?;
    let elapsed_total = start.elapsed();
    tracing::info!("compilation finished in {:?}", elapsed_total);

    let metadata = std::fs::metadata(&out_path).into_diagnostic()?;
    tracing::info!("{} bytes written", metadata.size());

    Ok(out_path)
}

pub fn handle(opts: BuildOpts) -> Result<()> {
    build(&opts).map(|_| ())
}
