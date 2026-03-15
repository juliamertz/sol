use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;

use crate::id;
use crate::lexer::source::Span;

id!(NodeId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub id: NodeId,
    pub span: Span,
    pub inner: Arc<str>,
}

impl Ident {
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl From<&Ident> for Arc<str> {
    fn from(val: &Ident) -> Self {
        val.inner.clone()
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

#[derive(Debug, Clone, Copy)]
pub enum OpKind {
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
pub struct Op {
    pub id: NodeId,
    pub span: Span,
    pub kind: OpKind,
}

/// A literal value within the source code
#[derive(Debug, Clone)]
pub enum LiteralKind {
    Str(Arc<str>),
    Int(i64),
    // Bool(bool),
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
    pub nodes: Arc<[Node]>,
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
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub ty: Option<Ty>,
    pub val: Expr,
}

#[derive(Debug, Clone)]
pub struct Ret {
    pub id: NodeId,
    pub span: Span,
    pub val: Expr,
}

#[derive(Debug, Clone)]
pub struct PrefixExpr {
    pub id: NodeId,
    pub span: Span,
    pub op: Op,
    pub rhs: Arc<Expr>,
}

#[derive(Debug, Clone)]
pub struct BinOp {
    pub id: NodeId,
    pub span: Span,
    pub lhs: Arc<Expr>,
    pub op: Op,
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
pub struct Fn {
    pub id: NodeId,
    pub span: Span,
    pub is_extern: bool,
    pub ident: Ident,
    pub params: Arc<[(Ident, Ty)]>,
    pub return_ty: Ty,
    pub body: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct Use {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub fields: Arc<[(Ident, Ty)]>,
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub body: Block,
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
    Prefix(PrefixExpr),
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
            Expr::Prefix(prefix_expr) => prefix_expr.span,
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
            Expr::Prefix(prefix_expr) => prefix_expr.id,
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
    Use(Use),
    Fn(Fn),
    StructDef(StructDef),
    Impl(Impl),
}

#[derive(Debug, Clone)]
pub enum Node {
    Expr(Expr),
    Stmnt(Stmnt),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub nodes: Arc<[Node]>,
}
