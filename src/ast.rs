pub type Identifier = String;

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
    IntLit(i64),
    BinOp(InfixExpr),
}

#[derive(Debug)]
pub enum Stmnt {
    Fn(Fn),
    Ret(Expr),
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
pub struct Fn {
    pub ident: Identifier,
    pub return_ty: Identifier,
    // pub args: Vec<Expr>,
    pub body: Block,
}
