use std::path::Display;

macro_rules! ast_derive {
    ($($item:item)*) => {
        $(
            #[derive(Debug, PartialEq, Eq, Clone)]
            #[cfg_attr(test, derive(serde::Serialize, serde::Deserialize))]
            $item
        )*
    };
}

pub type Ident = String;

ast_derive! {
    pub enum Node {
        Expr(Expr),
        Stmnt(Stmnt),
    }

    pub enum Expr {
        Ident(Ident),
        IntLit(i64),
        StringLit(String),
        Infix(InfixExpr),
        Call(CallExpr),
        If(If),
        List(List),
    }

    pub enum Stmnt {
        Fn(Fn),
        Ret(Ret),
        Use(Use),
        Let(Let),
    }

    pub enum Op {
        Eq,
        Add,
        Sub,
        Mul,
        Div,
        Lt,
        Gt,
        And,
        Or,

    }

    pub enum Type {
        Int,
        Bool,
        Str,
        Fn {
            r#extern: bool,
            args: Vec<Type>,
            returns: Box<Type>,
        },
        List(Box<Type>),
    }

    pub struct Block {
        pub nodes: Vec<Node>,
    }

    pub struct If {
        pub condition: Box<Expr>,
        pub consequence: Block,
        pub alternative: Option<Block>
    }

    pub struct List {
        pub items: Vec<Expr>,
    }

    pub struct Let {
        pub ident: Ident,
        pub ty: Option<Type>,
        pub val: Option<Expr>,
    }

    pub struct Ret {
        pub val: Expr,
    }

    pub struct InfixExpr {
        pub lhs: Box<Expr>,
        pub op: Op,
        pub rhs: Box<Expr>,
    }

    pub struct CallExpr {
        pub func: Box<Expr>,
        pub args: Vec<Expr>,
    }

    pub struct FnArg {
        pub ident: Ident,
        pub ty: Type,
    }

    pub struct Fn {
        pub r#extern: bool,
        pub name: Ident,
        pub args: Vec<FnArg>,
        pub return_ty: Type,
        pub body: Option<Block>,
    }

    pub struct Use {
        pub ident: Ident,
    }
}
