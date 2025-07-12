macro_rules! ast_impl {
    ($($item:item)*) => {
        $(
            #[derive(Debug, PartialEq, Eq, Clone)]
            #[cfg_attr(test, derive(serde::Serialize, serde::Deserialize))]
            $item
        )*
    };
}

pub type Ident = String;

ast_impl! {
    pub enum Node {
        Expr(Expr),
        Stmnt(Stmnt),
    }

    pub enum Expr {
        Ident(Ident),
        IntLit(i64),
        StringLit(String),
        Block(Block),
        Infix(InfixExpr),
        Prefix(PrefixExpr),
        Call(CallExpr),
        If(If),
        List(List),
        StructConstructor(StructConstructor),
    }

    pub enum Stmnt {
        Fn(Fn),
        Ret(Ret),
        Use(Use),
        Let(Let),
        StructDef(StructDef)
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
        Chain, // TODO: think of better name
    }

    pub enum Type {
        Int,
        Bool,
        Str,
        List((Box<Type>, Option<usize>)),
        Fn { args: Vec<Type>, returns: Box<Type>, is_extern: bool },
        Var(Ident),
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
        pub name: Ident,
        pub ty: Option<Type>,
        pub val: Option<Expr>,
    }

    pub struct Ret {
        pub val: Expr,
    }

    pub struct PrefixExpr {
        pub op: Op,
        pub rhs: Box<Expr>,
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

    pub struct Fn {
        pub is_extern: bool,
        pub name: Ident,
        pub args: Vec<(Ident, Type)>,
        pub return_ty: Type,
        pub body: Option<Block>,
    }

    pub struct Use {
        pub ident: Ident,
    }

    pub struct StructDef {
        pub ident: Ident,
        pub fields: Vec<(Ident, Type)>,
    }

    pub struct StructConstructor {
        pub ident: Ident,
        pub fields: Vec<(Ident, Expr)>,
    }
}
