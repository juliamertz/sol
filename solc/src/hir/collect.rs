use std::collections::HashMap;

use thiserror::Error;

use crate::ast::{Ident, Impl, Node, Stmnt};

#[derive(
    Error,
    // Diagnostic,
    Debug,
)]
pub enum CollectError {}

pub type Result<T> = std::result::Result<T, CollectError>;

#[derive(Debug, Default)]
pub struct Inventory<'ast> {
    impls: HashMap<Ident, Vec<&'ast Impl>>,
}

impl<'ast> Inventory<'ast> {
    pub fn take_impls(&mut self, ident: &Ident) -> Vec<&'ast Impl> {
        self.impls.remove(ident).unwrap_or_default()
    }
}

pub fn collect<'ast>(nodes: &'ast [Node]) -> Result<Inventory<'ast>> {
    let mut inventory = Inventory::default();

    for node in nodes.iter() {
        if let Node::Stmnt(Stmnt::Impl(inner)) = node {
            inventory
                .impls
                .entry(inner.ident.clone())
                .or_insert_with(|| Vec::new())
                .push(inner);
        }
    }

    Ok(inventory)
}
