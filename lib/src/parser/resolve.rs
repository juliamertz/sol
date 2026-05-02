use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::ast::{Module, PathSegment};
use crate::parser::Parser;
use crate::traits::AsStr;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] crate::parser::ParseError),
}

pub type Result<T, E = ResolveError> = std::result::Result<T, E>;

pub struct ModuleResolver {
    path: PathBuf,
    base: Module,
    inner: HashMap<PathBuf, Module>,
}

impl ModuleResolver {
    pub fn new(path: PathBuf, base: Module) -> Self {
        Self {
            path,
            base,
            inner: HashMap::default(),
        }
    }

    fn parse(&self, file_path: &Path) -> Result<Module> {
        let source = std::fs::read_to_string(&file_path)?;
        let mut parser = Parser::new(file_path, &source)?;
        Ok(parser.module()?)
    }

    pub fn try_resolve_all(&mut self) -> Result<()> {
        for stmnt in self.base.use_statements() {
            let file_path = stmnt
                .path
                .segments()
                .iter()
                .fold(self.path.clone(), |acc, segment| match segment {
                    PathSegment::Name(name) => acc.join(name.as_str()),
                });
            let module = self.parse(&file_path)?;
            self.inner.insert(file_path, module);
        }

        Ok(())
    }
}
