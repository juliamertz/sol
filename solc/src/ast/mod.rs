use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;

use either::Either;

use crate::ext::AsStr;
use crate::id;
use crate::lexer::source::Span;

id!(NodeId);

#[derive(Debug, Clone, Eq)]
pub struct Ident {
    pub id: NodeId,
    pub span: Span,
    pub inner: Arc<str>,
    pub is_extern: bool,
}

impl AsStr for Ident {
    fn as_str(&self) -> &str {
        &self.inner
    }
}

impl AsStr for &Ident {
    fn as_str(&self) -> &str {
        &self.inner
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.inner)
    }
}

impl Hash for Ident {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl PartialEq for Ident {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Name {
    pub span: Span,
    pub inner: Arc<str>,
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl AsStr for Name {
    fn as_str(&self) -> &str {
        &self.inner
    }
}

impl AsStr for &Name {
    fn as_str(&self) -> &str {
        &self.inner
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinOpKind {
    /// num == 10
    Eq,
    /// 4 + 2
    Add,
    /// 4 - 2
    Sub,
    /// 3 * 9
    Mul,
    /// 9 / 3
    Div,
    /// 0.5 < 1
    Lt,
    /// 1 > 0.5
    Gt,
    /// true and true
    And,
    /// false or true
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOpKind {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub struct Op<K> {
    pub span: Span,
    pub kind: K,
}

/// A literal value within the source code
#[derive(Debug, Clone)]
pub enum LiteralKind {
    Str(Arc<str>),
    Int(i128),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub id: NodeId,
    pub span: Span,
    pub kind: LiteralKind,
}

/// A type expression
#[derive(Debug, Clone)]
pub struct Ty {
    pub id: NodeId,
    pub span: Span,
    pub kind: TyKind,
}

#[derive(Debug, Clone, Copy)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
}

#[derive(Debug, Clone, Copy)]
pub enum UIntTy {
    U8,
    U16,
    U32,
    U64,
}

#[derive(Debug, Clone)]
pub enum TyKind {
    Int(IntTy),
    UInt(UIntTy),
    Bool,
    Str,
    List {
        inner: Arc<Ty>,
        size: Option<usize>,
    },
    Fn {
        params: Arc<[Ty]>,
        returns: Arc<Ty>,
        is_extern: bool,
    },
    Var(Ident),
}

/// A block of nodes, for example the body of a function or module
#[derive(Debug, Clone)]
pub struct Block {
    pub id: NodeId,
    pub span: Span,
    pub nodes: Arc<[Stmnt]>,
}

#[derive(Debug, Clone)]
pub struct IfElse {
    pub id: NodeId,
    pub span: Span,
    pub condition: Arc<Expr>,
    pub consequence: Block,
    pub alternative: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct List {
    pub id: NodeId,
    pub span: Span,
    pub items: Arc<[Expr]>,
}

#[derive(Debug, Clone)]
pub struct Let {
    pub span: Span,
    pub ident: Ident,
    pub ty: Option<Ty>,
    pub val: Expr,
}

#[derive(Debug, Clone)]
pub struct Ret {
    pub span: Span,
    pub val: Expr,
}

#[derive(Debug, Clone)]
pub struct Unary {
    pub id: NodeId,
    pub span: Span,
    pub op: Op<UnaryOpKind>,
    pub rhs: Arc<Expr>,
}

#[derive(Debug, Clone)]
pub struct BinOp {
    pub id: NodeId,
    pub span: Span,
    pub lhs: Arc<Expr>,
    pub op: Op<BinOpKind>,
    pub rhs: Arc<Expr>,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub id: NodeId,
    pub span: Span,
    pub func: Arc<Expr>,
    pub params: Arc<[Expr]>,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub id: NodeId,
    pub span: Span,
    pub expr: Arc<Expr>,
    pub idx: Arc<Expr>,
}

#[derive(Debug, Clone)]
pub enum FnKind {
    Local {
        params: Arc<[(Ident, Ty)]>,
        body: Block,
    },
    Extern {
        params: Arc<[(Name, Ty)]>,
    },
}

pub struct Param<'a> {
    pub key: &'a str,
    pub ty: &'a Ty,
    pub node_id: Option<NodeId>,
}

#[derive(Debug, Clone)]
pub struct Fn {
    pub span: Span,
    pub ident: Ident,
    pub kind: FnKind,
    pub return_ty: Ty,
}

impl Fn {
    pub fn params(&self) -> impl Iterator<Item = Param<'_>> {
        match self.kind {
            FnKind::Local { ref params, .. } => {
                Either::Left(params.iter().map(|(ident, ty)| Param {
                    key: ident.as_str(),
                    ty,
                    node_id: Some(ident.id),
                }))
            }
            FnKind::Extern { ref params } => Either::Right(params.iter().map(|(name, ty)| Param {
                key: name.as_str(),
                ty,
                node_id: None,
            })),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Use {
    pub span: Span,
    pub is_extern: bool,
    pub name: Name,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub span: Span,
    pub ident: Ident,
    pub fields: Arc<[(Name, Ty)]>,
}

// TODO:
// #[derive(Debug, Clone)]
// pub enum AssocItem {
//     Fn(Fn),
// }

#[derive(Debug, Clone)]
pub struct Impl {
    pub span: Span,
    pub ident: Ident,
    pub items: Arc<[Fn]>,
}

#[derive(Debug, Clone)]
pub struct MemberAccess {
    pub id: NodeId,
    pub span: Span,
    pub lhs: Arc<Expr>,
    pub ident: Ident,
}

#[derive(Debug, Clone)]
pub struct Constructor {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub fields: Arc<[(Ident, Expr)]>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(Ident),
    Literal(Literal),
    Block(Block),
    BinOp(BinOp),
    Unary(Unary),
    Call(CallExpr),
    Index(IndexExpr),
    IfElse(IfElse),
    List(List),
    Constructor(Constructor),
    MemberAccess(MemberAccess),
    Ref(Arc<Expr>), // TODO: why is this unused?
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Ident(ident) => ident.span,
            Expr::Literal(literal) => literal.span,
            Expr::Block(block) => block.span,
            Expr::BinOp(bin_op) => bin_op.span,
            Expr::Unary(unary) => unary.span,
            Expr::Call(call_expr) => call_expr.span,
            Expr::Index(index_expr) => index_expr.span,
            Expr::IfElse(if_else) => if_else.span,
            Expr::List(list) => list.span,
            Expr::Constructor(constructor) => constructor.span,
            Expr::MemberAccess(member_access) => member_access.span,
            Expr::Ref(expr) => expr.span(),
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            Expr::Ident(ident) => ident.id,
            Expr::Literal(literal) => literal.id,
            Expr::Block(block) => block.id,
            Expr::BinOp(bin_op) => bin_op.id,
            Expr::Unary(unary) => unary.id,
            Expr::Call(call_expr) => call_expr.id,
            Expr::Index(index_expr) => index_expr.id,
            Expr::IfElse(if_else) => if_else.id,
            Expr::List(list) => list.id,
            Expr::Constructor(constructor) => constructor.id,
            Expr::MemberAccess(member_access) => member_access.id,
            Expr::Ref(r#ref) => r#ref.id(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmnt {
    Let(Let),
    Ret(Ret),
    Expr(Expr),
}

impl Stmnt {
    pub fn span(&self) -> Span {
        match self {
            Stmnt::Let(inner) => inner.span,
            Stmnt::Ret(inner) => inner.span,
            Stmnt::Expr(inner) => inner.span(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Use(Use),
    Fn(Fn),
    StructDef(StructDef),
    Impl(Impl),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Arc<[Item]>,
}
