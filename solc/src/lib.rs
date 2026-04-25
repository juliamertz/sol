#![feature(test)]

pub mod ast;
pub mod codegen;
pub mod traits;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod type_checker;
#[macro_use]
pub mod interner;
pub mod mir;
pub mod number;
