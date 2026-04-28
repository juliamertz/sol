#![cfg_attr(test, feature(test))]
#![feature(portable_simd)]

#[macro_use]
mod interner;
mod number;
mod traits;

pub mod ast;
pub mod codegen;
pub mod hir;
pub mod lexer;
pub mod mir;
pub mod parser;
pub mod type_checker;
