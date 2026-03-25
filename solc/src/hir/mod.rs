use crate::ext::AsStr;
use crate::lexer::source::Span;
use crate::type_checker::{DefId, TypeId};
use crate::{ast, id};

mod lower;

#[doc(inline)]
pub use lower::*;

id!(HirId);

#[derive(Debug, Clone)]
pub struct Ident<'ast> {
    pub id: HirId,
    pub def_id: DefId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub inner: &'ast str,
}

#[derive(Debug, Clone)]
pub struct Name<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub inner: &'ast str,
}

#[derive(Debug, Clone)]
pub struct Literal<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub kind: &'ast ast::LiteralKind,
}

#[derive(Debug, Clone)]
pub struct Block<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub nodes: Box<[Stmnt<'ast>]>,
}

#[derive(Debug, Clone)]
pub struct BinOp<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub lhs: Box<Expr<'ast>>,
    pub op: &'ast ast::Op<ast::BinOpKind>,
    pub rhs: Box<Expr<'ast>>,
}

#[derive(Debug, Clone)]
pub struct Unary<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub op: &'ast ast::Op<ast::UnaryOpKind>,
    pub rhs: Box<Expr<'ast>>,
}

#[derive(Debug, Clone)]
pub struct Call<'ast> {
    pub id: HirId,
    pub def_id: DefId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub func: Box<Expr<'ast>>,
    pub params: Box<[Expr<'ast>]>,
}

#[derive(Debug, Clone)]
pub struct Index<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub expr: Box<Expr<'ast>>,
    pub idx: Box<Expr<'ast>>,
}

#[derive(Debug, Clone)]
pub struct IfElse<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub condition: Box<Expr<'ast>>,
    pub consequence: Block<'ast>,
    pub alternative: Option<Block<'ast>>,
}

#[derive(Debug, Clone)]
pub struct List<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub items: Box<[Expr<'ast>]>,
}

#[derive(Debug, Clone)]
pub struct Constructor<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub fields: Box<[(Ident<'ast>, Expr<'ast>)]>,
}

#[derive(Debug, Clone)]
pub struct MemberAccess<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub lhs: Box<Expr<'ast>>,
    pub ident: Ident<'ast>,
}

#[derive(Debug, Clone)]
pub enum Expr<'ast> {
    Ident(Ident<'ast>),
    Literal(Literal<'ast>),
    Block(Block<'ast>),
    BinOp(BinOp<'ast>),
    Unary(Unary<'ast>),
    Call(Call<'ast>),
    Index(Index<'ast>),
    IfElse(IfElse<'ast>),
    List(List<'ast>),
    Constructor(Constructor<'ast>),
    MemberAccess(MemberAccess<'ast>),
    Ref(Box<Expr<'ast>>),
}

#[derive(Debug, Clone)]
pub struct Let<'ast> {
    pub id: HirId,
    pub def_id: DefId,
    pub ty: TypeId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub val: Expr<'ast>,
}

#[derive(Debug, Clone)]
pub struct Ret<'ast> {
    pub id: HirId,
    pub ty: TypeId,
    pub span: Span,
    pub val: Expr<'ast>,
}

#[derive(Debug, Clone)]
pub struct Use<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub is_extern: bool,
    pub name: Name<'ast>,
}

#[derive(Debug, Clone)]
pub enum FnKind<'ast> {
    Local {
        params: Box<[(Ident<'ast>, TypeId)]>,
        body: Block<'ast>,
    },
    Extern {
        params: Box<[(Name<'ast>, TypeId)]>,
    },
}

#[derive(Debug, Clone)]
pub struct Fn<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub kind: FnKind<'ast>,
    pub return_ty: TypeId,
}

#[derive(Debug, Clone)]
pub struct StructDef<'ast> {
    pub id: HirId,
    pub span: &'ast Span,
    pub ident: Ident<'ast>,
    pub fields: Box<[(Name<'ast>, TypeId)]>,
    pub impls: Box<[&'ast ast::Impl]>, // TODO: This should probably also be lowered...
}

#[derive(Debug, Clone)]
pub enum Stmnt<'ast> {
    Let(Let<'ast>),
    Ret(Ret<'ast>),
    Expr(Expr<'ast>),
}

#[derive(Debug, Clone)]
pub enum Item<'ast> {
    Use(Use<'ast>),
    Fn(Fn<'ast>),
    StructDef(StructDef<'ast>),
}

#[derive(Debug, Clone)]
pub struct Module<'ast> {
    pub items: Box<[Item<'ast>]>,
}

impl AsStr for &Ident<'_> {
    fn as_str(&self) -> &str {
        self.inner
    }
}

impl AsStr for &Name<'_> {
    fn as_str(&self) -> &str {
        self.inner
    }
}

impl AsStr for Ident<'_> {
    fn as_str(&self) -> &str {
        self.inner
    }
}

impl AsStr for Name<'_> {
    fn as_str(&self) -> &str {
        self.inner
    }
}

impl Expr<'_> {
    pub fn type_id(&self) -> &TypeId {
        match self {
            Expr::Ident(ident) => &ident.ty,
            Expr::Literal(literal) => &literal.ty,
            Expr::Block(block) => &block.ty,
            Expr::BinOp(bin_op) => &bin_op.ty,
            Expr::Unary(unary) => &unary.ty,
            Expr::Call(call) => &call.ty,
            Expr::Index(index) => &index.ty,
            Expr::IfElse(if_else) => &if_else.ty,
            Expr::List(list) => &list.ty,
            Expr::Constructor(constructor) => &constructor.ty,
            Expr::MemberAccess(member_access) => &member_access.ty,
            Expr::Ref(expr) => expr.type_id(),
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            Expr::Ident(ident) => &ident.span,
            Expr::Literal(literal) => &literal.span,
            Expr::Block(block) => &block.span,
            Expr::BinOp(bin_op) => &bin_op.span,
            Expr::Unary(unary) => &unary.span,
            Expr::Call(call) => &call.span,
            Expr::Index(index) => &index.span,
            Expr::IfElse(if_else) => &if_else.span,
            Expr::List(list) => &list.span,
            Expr::Constructor(constructor) => &constructor.span,
            Expr::MemberAccess(member_access) => &member_access.span,
            Expr::Ref(expr) => expr.span(),
        }
    }
}
