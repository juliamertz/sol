use std::collections::HashMap;
use std::mem;

use miette::Diagnostic;
use thiserror::Error;

use crate::{
    ast::{Fn, Impl, Item, StructDef},
    ext::AsStr,
};

#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(solc::type_checker::collect))]
pub enum CollectError {}

pub type Result<T> = std::result::Result<T, CollectError>;

#[derive(Debug, Default)]
pub struct Inventory<'ast> {
    impls: HashMap<&'ast str, Vec<&'ast Impl>>, // TODO: Could be Map<Name, Impl>
    fns: Vec<&'ast Fn>,
    structs: Vec<&'ast StructDef>,
}

impl<'ast> Inventory<'ast> {
    pub fn take_impls(&mut self, name: impl AsStr) -> Vec<&'ast Impl> {
        self.impls.remove(name.as_str()).unwrap_or_default()
    }

    pub fn take_fns(&mut self) -> Vec<&'ast Fn> {
        mem::take(&mut self.fns)
    }

    pub fn take_structs(&mut self) -> Vec<&'ast StructDef> {
        mem::take(&mut self.structs)
    }
}

pub fn collect<'ast>(items: &'ast [Item]) -> Result<Inventory<'ast>> {
    let mut inventory = Inventory::default();

    for node in items.iter() {
        match node {
            Item::Impl(inner) => inventory
                .impls
                .entry(inner.ident.as_str())
                .or_insert_with(Vec::new)
                .push(inner),
            Item::Fn(inner) => inventory.fns.push(inner),
            Item::StructDef(inner) => inventory.structs.push(inner),
            Item::Use(_) => {}
        }
    }

    Ok(inventory)
}
