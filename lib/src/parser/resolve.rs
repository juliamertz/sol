use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;
use tracing::instrument;

use crate::ast::{Item, Module, Name, PathSegment};
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

#[derive(Debug)]
pub struct ModuleTree {
    root: ModuleNode,
}

impl ModuleTree {
    fn new(root: ModuleNode) -> Self {
        Self { root }
    }
}

#[derive(Debug)]
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
    pub fn new(ctx: Context) -> Self {
        Self { ctx }
    }

    fn resolve_path(&self, file_path: impl AsRef<Path>) -> Result<Module> {
        let content = std::fs::read_to_string(&file_path)?;
        let mut parser = Parser::new(file_path, &content)?;
        let module = parser.module()?;
        Ok(module)
    }

    fn resolve_node(&mut self, name: Name, module: Module) -> Result<ModuleNode> {
        let id = self.ctx.next_module();
        let mut node = ModuleNode::new(id, name, module);

        for item in node.module.use_statements() {}

        Ok(node)
    }

    pub fn resolve_tree(&mut self, name: Name, module: Module) -> Result<ModuleTree> {
        let root = self.resolve_node(name, module)?;
        Ok(ModuleTree::new(root))
    }

    pub fn finish(self) -> ModuleTree {
        ModuleTree { root: todo!() }
    }
}
