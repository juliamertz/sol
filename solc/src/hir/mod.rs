use crate::lexer::source::Span;
use crate::type_checker::interner::TypeId;
use crate::{ast, id};

mod collect;
mod lower;

#[doc(inline)]
pub use lower::*;

id!(HirId);

#[derive(Debug)]
pub struct Ident<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub inner: &'ast str,
}

#[derive(Debug)]
pub struct Literal<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub kind: &'ast ast::LiteralKind,
}

#[derive(Debug)]
pub struct Block<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub nodes: Box<[Node<'ast>]>,
}

#[derive(Debug)]
pub struct BinOp<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub lhs: Box<Expr<'ast>>,
    pub op: &'ast ast::Op,
    pub rhs: Box<Expr<'ast>>,
}

#[derive(Debug)]
pub struct Prefix<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub op: &'ast ast::Op,
    pub rhs: Box<Expr<'ast>>,
}

#[derive(Debug)]
pub struct Call<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub func: Box<Expr<'ast>>,
    pub params: Box<[Expr<'ast>]>,
}

#[derive(Debug)]
pub struct Index<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub expr: Box<Expr<'ast>>,
    pub idx: Box<Expr<'ast>>,
}

#[derive(Debug)]
pub struct IfElse<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub condition: Box<Expr<'ast>>,
    pub consequence: Block<'ast>,
    pub alternative: Option<Block<'ast>>,
}

#[derive(Debug)]
pub struct List<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub items: Box<[Expr<'ast>]>,
}

#[derive(Debug)]
pub struct Constructor<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub fields: Box<[(Ident<'ast>, Expr<'ast>)]>,
}

#[derive(Debug)]
pub struct MemberAccess<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub lhs: Box<Expr<'ast>>,
    pub ident: Ident<'ast>,
}

#[derive(Debug)]
pub enum ExprKind<'ast> {
    Ident(Ident<'ast>),
    Literal(Literal<'ast>),
    Block(Block<'ast>),
    BinOp(BinOp<'ast>),
    Prefix(Prefix<'ast>),
    Call(Call<'ast>),
    Index(Index<'ast>),
    IfElse(IfElse<'ast>),
    List(List<'ast>),
    Constructor(Constructor<'ast>),
    MemberAccess(MemberAccess<'ast>),
    Ref(Box<Expr<'ast>>),
}

#[derive(Debug)]
pub struct Expr<'ast> {
    pub kind: ExprKind<'ast>,
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug)]
pub struct Let<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub ty: Option<&'ast ast::Ty>,
    pub val: Expr<'ast>,
}

#[derive(Debug)]
pub struct Ret<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub val: Expr<'ast>,
}

#[derive(Debug)]
pub struct Use<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
}

#[derive(Debug)]
pub struct Fn<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub is_extern: bool,
    pub ident: Ident<'ast>,
    pub params: Box<[(Ident<'ast>, &'ast ast::Ty)]>,
    pub return_ty: &'ast ast::Ty,
    pub body: Option<Block<'ast>>,
}

#[derive(Debug)]
pub struct StructDef<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub fields: Box<[(Ident<'ast>, &'ast ast::Ty)]>,
    pub impls: Box<[&'ast ast::Impl]>, // TODO: This should probably also be lowered...
}

#[derive(Debug)]
pub enum Stmnt<'ast> {
    Let(Let<'ast>),
    Ret(Ret<'ast>),
    Use(Use<'ast>),
    Fn(Fn<'ast>),
    StructDef(StructDef<'ast>),
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
