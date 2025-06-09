mod c;
pub use c::C;

use crate::BuildOpts;
use miette::Result;
use std::path::PathBuf;

pub trait Emitter {
    type Input;

    fn emit(&mut self, ast: &Self::Input) -> String;
}

pub trait Compiler {
    fn build_exe(&self, src: &str, program: &str, opts: &BuildOpts) -> Result<PathBuf>;
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub enum ReleaseType {
    Fast,
    #[default]
    Debug,
}
