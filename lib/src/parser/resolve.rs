use std::collections::HashMap;
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;
use tracing::instrument;

use crate::ast::{Module, PathSegment};
use crate::parser::{Context, Parser};
use crate::traits::AsStr;

#[derive(Debug, Error, Diagnostic)]
pub enum ResolveError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] crate::parser::ParseError),
}

pub type Result<T, E = ResolveError> = std::result::Result<T, E>;

pub const FILE_EXTENSION: &str = "sol";

// TODO: we need to make it easy for the typechecker to turn paths 
// into a def_id from an item in one of these modules
#[derive(Debug)]
pub struct ModuleResolutions {
    map: HashMap<PathBuf, Module>,
}

pub struct ModuleResolver {
    ctx: Context,
    path: PathBuf,
    map: HashMap<PathBuf, Module>,
}

impl ModuleResolver {
    pub fn new(file_path: PathBuf, ctx: Context) -> Self {
        debug_assert!(file_path.is_file());
        Self {
            ctx,
            path: file_path
                .parent()
                .map(|dir| dir.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
            map: HashMap::default(),
        }
    }

    #[instrument(skip(self), err(Debug))]
    fn resolve_path(&mut self, file_path: &Path) -> Result<()> {
        if self.map.contains_key(file_path) {
            return Ok(());
        }

        let source = std::fs::read_to_string(&file_path)?;
        let mut parser = Parser::new_with(file_path, &source, self.ctx)?;
        let module = parser.module()?;
        self.ctx = parser.context();

        self.try_resolve_all(&module)?;
        self.map.insert(file_path.to_path_buf(), module);

        Ok(())
    }

    pub fn try_resolve_all(&mut self, module: &Module) -> Result<()> {
        for stmnt in module.use_statements() {
            let segments = stmnt.path.segments();
            let file_path =
                segments
                    .iter()
                    .enumerate()
                    .fold(self.path.clone(), |acc, (idx, segment)| match segment {
                        PathSegment::Name(name) => {
                            // this is janky
                            if idx == segments.len() - 1 {
                                acc.join(format!("{name}.{FILE_EXTENSION}"))
                            } else {
                                acc.join(name.as_str())
                            }
                        }
                    });

            self.resolve_path(&file_path)?;
        }

        Ok(())
    }

    pub fn finish(self) -> ModuleResolutions {
        ModuleResolutions { map: self.map }
    }
}
