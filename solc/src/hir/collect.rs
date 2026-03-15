use std::{collections::HashMap, mem};

use thiserror::Error;

use crate::ast::{Fn, Ident, Impl, Node, Stmnt, StructDef};

#[derive(
    Error,
    // Diagnostic,
    Debug,
)]
pub enum CollectError {}

pub type Result<T> = std::result::Result<T, CollectError>;

#[derive(Debug, Default)]
pub struct Inventory<'ast> {
    impls: HashMap<&'ast Ident, Vec<&'ast Impl>>,
    fns: Vec<&'ast Fn>,
    structs: Vec<&'ast StructDef>,
}

impl<'ast> Inventory<'ast> {
    pub fn take_impls(&mut self, ident: &Ident) -> Vec<&'ast Impl> {
        self.impls.remove(ident).unwrap_or_default()
    }

    pub fn take_fns(&mut self) -> Vec<&'ast Fn> {
        mem::take(&mut self.fns)
    }

    pub fn take_structs(&mut self) -> Vec<&'ast StructDef> {
        mem::take(&mut self.structs)
    }
}

pub fn collect<'ast>(nodes: &'ast [Node]) -> Result<Inventory<'ast>> {
    let mut inventory = Inventory::default();

    for node in nodes.iter() {
        match node {
            Node::Stmnt(Stmnt::Impl(inner)) => inventory
                .impls
                .entry(&inner.ident)
                .or_insert_with(Vec::new)
                .push(inner),
            Node::Stmnt(Stmnt::Fn(inner)) => inventory.fns.push(inner),
            Node::Stmnt(Stmnt::StructDef(inner)) => inventory.structs.push(inner),
            _ => {}
        }
    }

    Ok(inventory)
}
