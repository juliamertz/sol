use std::sync::Arc;

use solc_macros::Id;

use crate::ast::{
    BinOp, Block, CallExpr, Constructor, Fn, Ident, IfElse, Impl, IndexExpr, Let, List, Literal,
    MemberAccess, PrefixExpr, Ret, StructDef, Use,
};
use crate::lexer::source::Span;
use crate::type_checker::Type;

mod lower;

#[derive(Id, Debug, Clone, Copy)]
pub struct HirId(u32);

#[derive(Debug)]
pub enum ExprKind<'ast> {
    Ident(&'ast Ident),
    Literal(&'ast Literal),
    Block(&'ast Block),
    BinOp(&'ast BinOp),
    Prefix(&'ast PrefixExpr),
    Call(&'ast CallExpr),
    Index(&'ast IndexExpr),
    IfElse(&'ast IfElse),
    List(&'ast List),
    Constructor(&'ast Constructor),
    MemberAccess(&'ast MemberAccess),
    Ref(Box<Expr<'ast>>),
}

#[derive(Debug)]
pub struct Expr<'ast> {
    pub hir_id: HirId,
    pub kind: ExprKind<'ast>,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug)]
pub enum Stmnt {
    Let(Let),
    Ret(Ret),
    Use(Use),
    Fn(Fn),
    StructDef(StructDef),
    Impl(Impl),
}

#[derive(Debug)]
pub enum Node<'ast> {
    Expr(Expr<'ast>),
    Stmnt(Stmnt),
}

#[derive(Debug)]
pub struct Module<'ast> {
    pub nodes: Box<[Node<'ast>]>,
}
