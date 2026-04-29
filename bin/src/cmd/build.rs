use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};

use lib::codegen::qbe;
use lib::{hir, mir, parser, type_checker};

#[derive(clap::Args)]
pub struct BuildOpts {
    /// Path to source code
    pub file_path: PathBuf,

    /// Path to directory which all build artifacts get written to
    #[arg(short, long, default_value = "out")]
    pub outdir: PathBuf,

    /// Whether to clean up build artifacts
    #[arg(short, long)]
    pub cleanup: bool,
}

pub fn build(opts: &BuildOpts) -> Result<PathBuf> {
    let file_path = opts.file_path.as_path();
    let content = std::fs::read_to_string(file_path).unwrap();
    let _name = file_path.to_string_lossy();

    let mut parser = parser::Parser::new(file_path.to_owned(), &content)?;
    let module_ast = parser.parse()?;

    let mut env = type_checker::TypeEnv::new(parser.source());
    let mut scope = type_checker::Scope::default();
    type_checker::check_module(&module_ast, &mut env, &mut scope)?;

    let module_hir = hir::lower_module(&module_ast, &mut env)?;
    let module_mir = mir::lower_module(&module_hir, &env)?;

    let mut qbe_builder = qbe::lower::Builder::new(&env);
    let qbe_module = qbe_builder.lower_module(&module_mir)?;

    let _ = fs::create_dir(&opts.outdir);
    let compiler = qbe::compile::Compiler::new(&opts.outdir);
    let out_path = compiler.ir_to_bin(&qbe_module)?;

    Ok(out_path)
}

pub fn handle(opts: BuildOpts) -> Result<()> {
    let bin_path = build(&opts)?;
    let metadata = std::fs::metadata(&bin_path).into_diagnostic()?;
    println!("{} bytes written to {bin_path:?}", metadata.size());
    Ok(())
}
