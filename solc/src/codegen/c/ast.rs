use std::fmt::{self, Write};

use crate::ext::Boxed;

#[derive(Debug, Clone)]
pub enum CType {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Bool,
    CharPtr,
    Void,
    Named(String),
    Ptr(Box<CType>),
}

impl CType {
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    pub fn ptr(inner: CType) -> Self {
        Self::Ptr(inner.boxed())
    }
}

impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int8 => f.write_str("int8_t"),
            Self::Int16 => f.write_str("int16_t"),
            Self::Int32 => f.write_str("int32_t"),
            Self::Int64 => f.write_str("int64_t"),
            Self::UInt8 => f.write_str("uint8_t"),
            Self::UInt16 => f.write_str("uint16_t"),
            Self::UInt32 => f.write_str("uint32_t"),
            Self::UInt64 => f.write_str("uint64_t"),
            Self::Bool => f.write_str("bool"),
            Self::CharPtr => f.write_str("char *"),
            Self::Void => f.write_str("void"),
            Self::Named(name) => f.write_str(name),
            Self::Ptr(inner) => write!(f, "{inner} *"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CExpr {
    Ident(String),
    IntLit(i128),
    StrLit(String),
    BinOp {
        lhs: Box<CExpr>,
        op: &'static str,
        rhs: Box<CExpr>,
    },
    Prefix {
        op: &'static str,
        expr: Box<CExpr>,
    },
    Call {
        func: Box<CExpr>,
        args: Vec<CExpr>,
    },
    Member {
        expr: Box<CExpr>,
        field: String,
    },
    CompoundLit {
        ty: String,
        fields: Vec<(String, CExpr)>,
    },
    StmtExpr(Vec<CStmt>),
    Sizeof(CType),
    AddrOf(Box<CExpr>),
}

impl CExpr {
    pub fn ident(name: impl Into<String>) -> Self {
        Self::Ident(name.into())
    }

    pub fn int(val: i128) -> Self {
        Self::IntLit(val)
    }

    pub fn str(val: impl Into<String>) -> Self {
        Self::StrLit(val.into())
    }

    pub fn call(func: CExpr, args: Vec<CExpr>) -> Self {
        Self::Call {
            func: func.boxed(),
            args,
        }
    }

    pub fn member(self, field: impl Into<String>) -> Self {
        Self::Member {
            expr: self.boxed(),
            field: field.into(),
        }
    }

    pub fn binop(self, op: &'static str, rhs: CExpr) -> Self {
        Self::BinOp {
            lhs: self.boxed(),
            op,
            rhs: rhs.boxed(),
        }
    }

    pub fn prefix(op: &'static str, expr: CExpr) -> Self {
        Self::Prefix {
            op,
            expr: expr.boxed(),
        }
    }

    pub fn addr_of(self) -> Self {
        Self::AddrOf(self.boxed())
    }

    pub fn compound_lit(ty: impl Into<String>, fields: Vec<(String, CExpr)>) -> Self {
        Self::CompoundLit {
            ty: ty.into(),
            fields,
        }
    }

    pub fn sizeof(ty: CType) -> Self {
        Self::Sizeof(ty)
    }

    pub fn stmt_expr(stmts: Vec<CStmt>) -> Self {
        Self::StmtExpr(stmts)
    }
}

impl fmt::Display for CExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(name) => f.write_str(name),
            Self::IntLit(val) => write!(f, "{val}"),
            Self::StrLit(val) => write!(f, "\"{val}\""),
            Self::BinOp { lhs, op, rhs } => write!(f, "{lhs}{op}{rhs}"),
            Self::Prefix { op, expr } => write!(f, "{op}{expr}"),
            Self::Call { func, args } => {
                write!(f, "{func}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{arg}")?;
                }
                f.write_char(')')
            }
            Self::Member { expr, field } => write!(f, "{expr}.{field}"),
            Self::CompoundLit { ty, fields } => {
                write!(f, "({ty}){{")?;
                for (name, val) in fields {
                    write!(f, ".{name}={val},")?;
                }
                f.write_char('}')
            }
            Self::StmtExpr(stmts) => {
                f.write_str("({")?;
                for stmt in stmts {
                    write!(f, "{stmt}")?;
                }
                f.write_str("})")
            }
            Self::Sizeof(ty) => write!(f, "sizeof({ty})"),
            Self::AddrOf(expr) => write!(f, "&{expr}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CStmt {
    VarDecl {
        ty: CType,
        name: String,
        init: Option<CExpr>,
    },
    Return(CExpr),
    Expr(CExpr),
    StmntExpr(CExpr),
    If {
        cond: CExpr,
        then: Vec<CStmt>,
        else_: Option<Vec<CStmt>>,
    },
}

impl CStmt {
    pub fn var(ty: CType, name: impl Into<String>, init: CExpr) -> Self {
        Self::VarDecl {
            ty,
            name: name.into(),
            init: Some(init),
        }
    }

    pub fn ret(expr: CExpr) -> Self {
        Self::Return(expr)
    }

    pub fn expr(expr: CExpr) -> Self {
        Self::Expr(expr)
    }

    pub fn if_(cond: CExpr, then: Vec<CStmt>) -> Self {
        Self::If {
            cond,
            then,
            else_: None,
        }
    }

    pub fn if_else(cond: CExpr, then: Vec<CStmt>, else_: Vec<CStmt>) -> Self {
        Self::If {
            cond,
            then,
            else_: Some(else_),
        }
    }
}

impl fmt::Display for CStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VarDecl { ty, name, init } => match init {
                Some(expr) => write!(f, "{ty} {name}={expr};"),
                None => write!(f, "{ty} {name};"),
            },
            Self::Return(expr) => write!(f, "return {expr};"),
            Self::Expr(expr) => write!(f, "{expr};"),
            Self::StmntExpr(expr) => write!(f, "({{{expr}}});"),
            Self::If { cond, then, else_ } => {
                write!(f, "if({cond}){{")?;
                for stmt in then {
                    write!(f, "{stmt}")?;
                }
                f.write_char('}')?;
                if let Some(else_stmts) = else_ {
                    f.write_str("else{")?;
                    for stmt in else_stmts {
                        write!(f, "{stmt}")?;
                    }
                    f.write_char('}')?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CItem {
    Include(CInclude),
    TypedefStruct {
        name: String,
        fields: Vec<(CType, String)>,
    },
    FnDef {
        ret: CType,
        name: String,
        params: Vec<(CType, String)>,
        body: Vec<CStmt>,
    },
}

#[derive(Debug, Clone)]
pub enum CInclude {
    System(String),
    Local(String),
}

impl CItem {
    pub fn include_system(path: impl Into<String>) -> Self {
        Self::Include(CInclude::System(path.into()))
    }

    pub fn include_local(path: impl Into<String>) -> Self {
        Self::Include(CInclude::Local(path.into()))
    }

    pub fn typedef_struct(name: impl Into<String>, fields: Vec<(CType, String)>) -> Self {
        Self::TypedefStruct {
            name: name.into(),
            fields,
        }
    }

    pub fn fn_def(
        ret: CType,
        name: impl Into<String>,
        params: Vec<(CType, String)>,
        body: Vec<CStmt>,
    ) -> Self {
        Self::FnDef {
            ret,
            name: name.into(),
            params,
            body,
        }
    }
}

impl fmt::Display for CItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Include(CInclude::System(path)) => writeln!(f, "#include <{path}>"),
            Self::Include(CInclude::Local(path)) => writeln!(f, "#include \"{path}\""),
            Self::TypedefStruct { name, fields } => {
                write!(f, "typedef struct {name}{{")?;
                for (ty, field_name) in fields {
                    write!(f, "{ty} {field_name};")?;
                }
                write!(f, "}}{name};")
            }
            Self::FnDef {
                ret,
                name,
                params,
                body,
            } => {
                write!(f, "{ret} {name}(")?;
                for (i, (ty, param_name)) in params.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{ty} {param_name}")?;
                }
                f.write_str("){")?;
                for stmt in body {
                    write!(f, "{stmt}")?;
                }
                f.write_char('}')
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CTranslationUnit {
    pub items: Vec<CItem>,
}

impl CTranslationUnit {
    pub fn push(&mut self, item: CItem) {
        self.items.push(item);
    }
}

impl fmt::Display for CTranslationUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.items {
            write!(f, "{item}")?;
        }
        Ok(())
    }
}
