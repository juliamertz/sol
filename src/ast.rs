#[derive(Debug)]
pub enum Node {
    Expr(Expr),
}

#[derive(Debug)]
pub enum Expr {
    IntLit(i64),
    BinOp(BinOp),
}

#[derive(Debug)]
pub enum Op {
    Add,
    Sub,
}

#[derive(Debug)]
pub struct BinOp {
    pub lhs: Box<Expr>,
    pub op: Op,
    pub rhs: Box<Expr>,
}
