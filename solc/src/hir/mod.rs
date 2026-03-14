use std::sync::Arc;

use solc_macros::Id;

use crate::ast;
use crate::lexer::source::Span;
use crate::type_checker::Type;
use crate::type_checker::interner::TypeId;

mod collect;
mod lower;

#[doc(inline)]
pub use collect::*;
#[doc(inline)]
pub use lower::*;

#[derive(Id, Debug, Clone, Copy)]
pub struct HirId(u32);

#[derive(Debug, Clone)]
pub struct Block<'ast> {
    pub id: HirId,
    pub nodes: Arc<[Node<'ast>]>,
}

#[derive(Debug)]
pub enum ExprKind<'ast> {
    Ident(&'ast ast::Ident),
    Literal(&'ast ast::Literal),
    Block(&'ast ast::Block),
    BinOp(&'ast ast::BinOp),
    Prefix(&'ast ast::PrefixExpr),
    Call(&'ast ast::CallExpr),
    Index(&'ast ast::IndexExpr),
    IfElse(&'ast ast::IfElse),
    List(&'ast ast::List),
    Constructor(&'ast ast::Constructor),
    MemberAccess(&'ast ast::MemberAccess),
    Ref(Box<Expr<'ast>>),
}

#[derive(Debug)]
pub struct Expr<'ast> {
    pub hir_id: HirId,
    pub kind: ExprKind<'ast>,
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug)]
pub enum Stmnt<'ast> {
    Let(&'ast ast::Let),
    Ret(&'ast ast::Ret),
    Use(&'ast ast::Use),
    Fn(&'ast ast::Fn),
    StructDef {
        def: &'ast ast::StructDef,
        impls: Box<[&'ast ast::Impl]>,
    },
}

#[derive(Debug)]
pub enum Node<'ast> {
    Expr(Expr<'ast>),
    Stmnt(Stmnt<'ast>),
}

#[derive(Debug)]
pub struct Module<'ast> {
    pub nodes: Box<[Node<'ast>]>,
}
