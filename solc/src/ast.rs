use std::fmt::Display;

use serde::{Deserialize, Serialize};
use solc_macros::Id;

pub type Span = miette::SourceSpan;

#[derive(Id, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NodeId(u32);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Ident {
    pub id: NodeId,
    pub span: Span,
    pub inner: String,
}

impl Ident {
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// struct.field or struct.method()
    Chain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Op {
    pub id: NodeId,
    pub span: Span,
    pub kind: OpKind,
}

/// A literal value within the source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralKind {
    Str(String),
    Int(u64),
    // Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Literal {
    pub id: NodeId,
    pub span: Span,
    pub kind: LiteralKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ty {
    pub id: NodeId,
    pub span: Span,
    pub kind: TyKind,
}

/// Type expression such as `List<Int>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TyKind {
    Int,
    Bool,
    Str,
    List {
        inner: Box<Ty>,
        size: Option<usize>,
    },
    Fn {
        args: Vec<Ty>,
        returns: Box<Ty>,
        is_extern: bool,
    },
    Var(Ident),
}

/// A block of nodes, for example the body of a function or module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: NodeId,
    pub span: Span,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct If {
    pub id: NodeId,
    pub span: Span,
    pub condition: Box<Expr>,
    pub consequence: Block,
    pub alternative: Option<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: NodeId,
    pub span: Span,
    pub items: Vec<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Let {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub ty: Option<Ty>,
    pub val: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ret {
    pub id: NodeId,
    pub span: Span,
    pub val: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixExpr {
    pub id: NodeId,
    pub span: Span,
    pub op: Op,
    pub rhs: Box<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinOp {
    pub id: NodeId,
    pub span: Span,
    pub lhs: Box<Expr>,
    pub op: Op,
    pub rhs: Box<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpr {
    pub id: NodeId,
    pub span: Span,
    pub func: Box<Expr>,
    pub params: Vec<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexExpr {
    pub id: NodeId,
    pub span: Span,
    pub expr: Box<Expr>,
    pub idx: Box<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fn {
    pub id: NodeId,
    pub span: Span,
    pub is_extern: bool,
    pub ident: Ident,
    pub params: Vec<(Ident, Ty)>,
    pub return_ty: Ty,
    pub body: Option<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Use {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub fields: Vec<(Ident, Ty)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impl {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub body: Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constructor {
    pub id: NodeId,
    pub span: Span,
    pub ident: Ident,
    pub fields: Vec<(Ident, Expr)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Ident(Ident),
    Literal(Literal),
    Block(Block),
    BinOp(BinOp),
    Prefix(PrefixExpr),
    Call(CallExpr),
    Index(IndexExpr),
    IfElse(If),
    List(List),
    Constructor(Constructor),
    Ref(Box<Expr>),
    /// Used for inserting identifiers that will not be mangled in the final output
    /// For internal use during codegen or when using extern symbols
    RawIdent(Ident),
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
            Expr::Ref(expr) => expr.span(),
            Expr::RawIdent(_) => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmnt {
    Fn(Fn),
    Ret(Ret),
    Use(Use),
    Let(Let),
    StructDef(StructDef),
    Impl(Impl),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    Expr(Expr),
    Stmnt(Stmnt),
}
