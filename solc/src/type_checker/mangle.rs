use std::fmt::Display;

use crate::ast::Ident;

#[derive(Debug)]
pub enum Mangle<'a> {
    AssocItem(&'a Ident, &'a Ident),
}

impl Display for Mangle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mangle::AssocItem(def, item) => write!(f, "_{def}_{item}"),
        }
    }
}
