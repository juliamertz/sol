mod c;

pub use c::{C, CCOpts};

pub trait Emitter {
    type Input;

    fn emit(&mut self, ast: &Self::Input) -> String;
}

pub trait Compiler {
    type Opts;

    fn build_exe(&self, src: &str, program: &str, opts: Self::Opts) -> std::path::PathBuf;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum ReleaseType {
    Fast,
    #[default]
    Debug,
}
