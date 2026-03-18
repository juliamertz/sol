// required for miette `Diagnostic` derive
// see: https://github.com/rust-lang/rust/issues/147648
#![allow(unused_assignments)]

pub mod ast;
pub mod codegen;
pub mod ext;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod type_checker;
#[macro_use]
pub mod interner;
