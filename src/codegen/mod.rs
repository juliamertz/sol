mod c;

use crate::BuildOpts;
pub use c::C;

pub trait Emitter {
    type Input;

    fn emit(&mut self, ast: &Self::Input) -> String;
}

pub trait Compiler {
    fn build_exe(&self, src: &str, program: &str, opts: &BuildOpts) -> std::path::PathBuf;
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub enum ReleaseType {
    Fast,
    #[default]
    Debug,
}
