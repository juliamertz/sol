mod c;
pub use c::C;

use crate::BuildOpts;
use crate::analyzer::TypeEnv;

use miette::Result;
use std::borrow::Cow;
use std::path::PathBuf;

pub trait Emitter {
    type Input;

    fn emit(&mut self, env: TypeEnv, ast: &Self::Input) -> String;
}

pub trait Compiler {
    fn build_exe(&self, src: &str, program: &str, opts: &BuildOpts) -> Result<PathBuf>;

    /// Optional formatter implementation for debugging codegen output
    fn format<'src>(&self, source: &'src str) -> Cow<'src, str> {
        Cow::Borrowed(source)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub enum ReleaseType {
    Fast,
    #[default]
    Debug,
}

pub fn quote(text: impl AsRef<str>) -> String {
    format!("\"{}\"", text.as_ref())
}
