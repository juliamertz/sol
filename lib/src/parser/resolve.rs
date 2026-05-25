use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;
use tracing::instrument;

use crate::ast::{Module, PathSegment};
use crate::parser::{Context, Parser};
use crate::traits::AsStr;

pub const FILE_EXTENSION: &str = "sol";

#[derive(Debug, Error, Diagnostic)]
pub enum ResolveError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] crate::parser::ParseError),
}

pub type Result<T, E = ResolveError> = std::result::Result<T, E>;

pub type ModuleName = Arc<str>;

id!(ModuleId);

#[derive(Debug)]
pub struct ModuleTree {
    root: ModuleNode,
}

impl ModuleTree {
    const fn new(root: ModuleNode) -> Self {
        Self { root }
    }

    pub const fn root(&self) -> &ModuleNode {
        &self.root
    }
}

#[derive(Debug)]
pub struct ModuleNode {
    id: ModuleId,
    name: ModuleName,
    module: Module,
    children: BTreeMap<ModuleName, Module>,
}

impl ModuleNode {
    fn new(id: ModuleId, name: ModuleName, module: Module) -> Self {
        Self {
            id,
            name,
            module,
            children: BTreeMap::new(),
        }
    }

    pub const fn name(&self) -> &ModuleName {
        &self.name
    }

    pub const fn children(&self) -> &BTreeMap<ModuleName, Module> {
        &self.children
    }
}

pub struct ModuleResolver {
    ctx: Context,
    dir: PathBuf,
}

impl ModuleResolver {
    pub fn new(ctx: Context, dir: PathBuf) -> Self {
        Self { ctx, dir }
    }

    #[instrument(skip_all, err(Debug), fields(file_path = ?file_path.as_ref()))]
    fn resolve_path(&self, file_path: impl AsRef<Path>) -> Result<Module> {
        let content = std::fs::read_to_string(&file_path)?;
        let mut parser = Parser::new(file_path, &content)?;
        let module = parser.module()?;
        Ok(module)
    }

    #[instrument(skip(self), err(Debug))]
    fn resolve_node(&mut self, name: ModuleName, module: Module) -> Result<ModuleNode> {
        let id = self.ctx.next_module();
        let mut node = ModuleNode::new(id, name, module);

        for item in node.module.use_statements() {
            // TODO: support more complex paths
            let PathSegment::Named(name) = item.path.segments().into_iter().next().unwrap();
            let file_name = format!("{}.{FILE_EXTENSION}", name.as_str());
            let file_path = self.dir.join(file_name);

            let module = self.resolve_path(file_path)?;
            node.children.insert(name.inner.clone(), module);
        }

        Ok(node)
    }

    #[instrument(skip(self, module), err(Debug))]
    pub fn resolve_tree(&mut self, name: ModuleName, module: Module) -> Result<ModuleTree> {
        let root = self.resolve_node(name, module)?;
        Ok(ModuleTree::new(root))
    }
}
