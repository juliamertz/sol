use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;
use tracing::instrument;

use crate::ast::{Module, Name, PathSegment};
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

id!(ModuleId);

pub struct ModuleTree {
    root: ModuleNode,
}

pub struct ModuleNode {
    id: ModuleId,
    name: Name,
    module: Module,
    children: BTreeMap<Name, Module>,
}

impl ModuleNode {
    fn new(id: ModuleId, name: Name, module: Module) -> Self {
        Self {
            id,
            name,
            module,
            children: BTreeMap::new(),
        }
    }
}

pub struct ModuleResolver {
    ctx: Context,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self { ctx }
    }

    pub fn finish(self) -> ModuleTree {
        ModuleTree { root: todo!() }
    }
}
