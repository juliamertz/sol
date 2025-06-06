pub type Ident = String;

pub type Type = String;

#[derive(Debug)]
pub enum Node {
    Expr(Expr),
    Stmnt(Stmnt),
}

#[derive(Debug)]
pub struct Block {
    pub nodes: Vec<Node>,
}

#[derive(Debug)]
pub enum Expr {
    Ident(Ident),
    IntLit(i64),
    StringLit(String),
    InfixExpr(InfixExpr),
    CallExpr(CallExpr),
}

#[derive(Debug)]
pub enum Stmnt {
    Fn(Fn),
    Ret(Expr),
    Use(Use),
}

#[derive(Debug)]
pub enum Op {
    Add,
    Sub,
}

#[derive(Debug)]
pub struct InfixExpr {
    pub lhs: Box<Expr>,
    pub op: Op,
    pub rhs: Box<Expr>,
}

#[derive(Debug)]
pub struct CallExpr {
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug)]
pub struct FnArg {
    pub ident: Ident,
    pub ty: Type,
}

#[derive(Debug)]
pub struct Fn {
    pub ident: Ident,
    pub args: Vec<FnArg>,
    pub return_ty: Ident,
    pub body: Block,
}

#[derive(Debug)]
pub struct Use {
    pub ident: Ident,
}
