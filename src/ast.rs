pub type Ident = String;

pub type Ty = String;

#[derive(Debug, PartialEq, Eq)]
pub enum Node {
    Expr(Expr),
    Stmnt(Stmnt),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    pub nodes: Vec<Node>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct If {
    pub condition: Box<Expr>,
    pub consequence: Block,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Ident(Ident),
    IntLit(i64),
    StringLit(String),
    InfixExpr(InfixExpr),
    CallExpr(CallExpr),
    If(If),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Stmnt {
    Fn(Fn),
    Ret(Ret),
    Use(Use),
    Let(Let),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Let{
    pub ident: Ident,
    pub ty: Ty,
    pub val: Option<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ret{
    pub val: Expr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Op {
    Eq,
    Add,
    Sub,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InfixExpr {
    pub lhs: Box<Expr>,
    pub op: Op,
    pub rhs: Box<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CallExpr {
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FnArg {
    pub ident: Ident,
    pub ty: Ty,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Fn {
    pub ident: Ident,
    pub args: Vec<FnArg>,
    pub return_ty: Ident,
    pub body: Block,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Use {
    pub ident: Ident,
}
