pub mod c;
pub mod qbe;

use crate::hir;
use crate::type_checker::TypeEnv;

use std::borrow::Cow;
use std::path::PathBuf;

pub trait Emitter {
    fn emit(&mut self, env: TypeEnv, input: &hir::Module<'_>) -> String;
}

pub struct BuildOpts {
    pub release: bool,
    pub outdir: PathBuf,
    pub cleanup: bool,
}

pub trait Compiler {
    type Err;

    fn build_exe(&self, src: &str, program: &str, opts: &BuildOpts) -> Result<PathBuf, Self::Err>;

    /// Optional formatter implementation for debugging codegen output
    fn format<'src>(&self, source: &'src str) -> Cow<'src, str> {
        Cow::Borrowed(source)
    }
}

pub fn quote(text: impl AsRef<str>) -> String {
    format!("\"{}\"", text.as_ref())
}
