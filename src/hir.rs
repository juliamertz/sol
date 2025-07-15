// a high level IR that closely maps to the AST
// but with type annotations and other useful information

use std::collections::HashMap;

use miette::{Context, Diagnostic, IntoDiagnostic, Result, SourceSpan};
use thiserror::Error;

use crate::ast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Any, // TODO: We should remove this, but for now we can use it as a crutch
    Int,
    Bool,
    Str,
    List((Box<Type>, Option<usize>)),
    Ptr(Box<Type>),
    Fn {
        is_extern: bool,
        args: Vec<Type>,
        returns: Box<Type>,
    },
    Struct {
        ident: String,
        fields: Vec<(String, Type)>,
    },
    // TODO: include sourcespan so we can have nicer debug messages
    Var(String),
}

impl Type {
    fn list(ty: Type, size: Option<usize>) -> Self {
        Self::List((Box::new(ty), size))
    }

    fn ptr(ty: Type) -> Self {
        Self::Ptr(Box::new(ty))
    }

    fn is_concrete(&self) -> bool {
        !matches!(self, Self::Var(_))
    }
}

impl From<&ast::Type> for Type {
    fn from(value: &ast::Type) -> Self {
        match value {
            ast::Type::Int => Self::Int,
            ast::Type::Bool => Self::Bool,
            ast::Type::Str => Self::Str,
            ast::Type::List((ty, size)) => Self::list(Type::from(&(**ty)), *size),
            ast::Type::Fn {
                args,
                returns,
                is_extern,
            } => todo!(),
            ast::Type::Var(name) => Self::Var(name.clone()),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum TypeError {
    #[error("Type mismatch between {0:?} and {1:?}")]
    TypeMismatch(Type, Type),

    #[error("No such variable: '{name}'")]
    UndefinedVariable { name: String },
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Var,
    Fn,
    Field,
    Struct,
    Method,
    Param,
}

type SymbolId = u32;

#[derive(Debug, Clone)]
struct Symbol {
    kind: SymbolKind,
    name: String,
    ty: Type,
    id: SymbolId,
}

type Level = u32;

#[derive(Debug, Clone, Default)]
pub struct Scope {
    bindings: HashMap<String, SymbolId>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Expr(Expr),
    Stmnt(Stmnt),
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    StrLit(String),
    Var {
        id: SymbolId,
        ty: Type,
    },
    BinOp {
        lhs: Box<Expr>,
        op: ast::Op,
        rhs: Box<Expr>,
        ty: Type,
    },
    Unary {
        op: ast::Op,
        rhs: Box<Expr>,
        ty: Type,
    },
    Block {
        nodes: Vec<Node>,
        ty: Type,
    },
    Call {
        id: SymbolId,
        params: Vec<SymbolId>,
        ty: Type,
    },
    Index {
        id: SymbolId,
        idx: usize,
        ty: Type,
    },
    IfElse {
        condition: Box<Expr>,
        consequence: Vec<Node>,
        alternative: Option<Vec<Node>>,
        ty: Type,
    },
    Constructor {
        id: SymbolId,
        fields: Vec<(String, Expr)>,
    },
    Ref(Box<Expr>),
    Deref(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmnt {
    Let {
        id: SymbolId,
        val: Option<Expr>,
        ty: Type,
    },
    Fn {
        id: SymbolId,
        r#extern: bool,
        params: Vec<SymbolId>,
        body: Vec<Node>,
        ty: Type,
    },
    Ret {
        implicit: bool,
        val: Option<Expr>,
        ty: Type,
    },
    Use {
        path: Vec<String>,
        // ty: Type,
    },
    Struct {
        id: SymbolId,
        ty: Type,
        impls: Scope,
    },
}

#[derive(Default)]
pub struct HirBuilder {
    symbols: Vec<Symbol>,
}

impl HirBuilder {
    fn new_symbol(&mut self, name: impl ToString, kind: SymbolKind) -> &Symbol {
        let id = self.symbols.len();
        self.symbols.push(Symbol {
            id: id.try_into().unwrap(),
            kind,
            name: name.to_string(),
            ty: Type::Any,
        });
        unsafe { self.symbols.get_unchecked(id) }
    }

    fn infer(&self, node: &ast::Node) -> Result<Type> {
        match node {
            ast::Node::Expr(expr) => self.infer_expr(expr),
            ast::Node::Stmnt(stmnt) => self.infer_stmnt(stmnt),
        }
    }

    fn infer_expr(&self, expr: &ast::Expr) -> Result<Type> {
        match expr {
            ast::Expr::IntLit(_) => Ok(Type::Int),
            ast::Expr::StrLit(_) => Ok(Type::Str),

            ast::Expr::Block(block) => {
                todo!("check block expressions")
            }

            ast::Expr::Prefix(prefix_expr) => {
                todo!();
            }

            ast::Expr::Infix(infix_expr) => {
                let lhs = self.infer_expr(&infix_expr.lhs)?;
                let rhs = self.infer_expr(&infix_expr.rhs)?;
                if lhs != rhs {
                    return Err(TypeError::TypeMismatch(lhs, rhs).into());
                }
                Ok(lhs)
            }

            ast::Expr::List(list) => {
                let mut items = list.items.iter();
                let expected_ty = items
                    .next()
                    .map(|item| self.infer_expr(item))
                    .unwrap_or(Ok(Type::Any))?; // TODO: how do we handle inferring empty lists?

                for item in items {
                    let ty = self.infer_expr(item)?;
                    if ty != expected_ty {
                        return Err(TypeError::TypeMismatch(ty, expected_ty))
                            .into_diagnostic()
                            .wrap_err("List type mismatch");
                    }
                }

                Ok(Type::list(expected_ty, None)) // TODO: fixed size lists
            }

            // ast::Expr::Ident(name) => env
            //     .get(name)
            //     .cloned()
            //     .ok_or_else(|| AnalyzeError::UndefinedVariable { name: name.clone() }.into()),

            ast::Expr::Call(call_expr) => match self.infer_expr(&call_expr.func)? {
                Type::Fn { returns, .. } => Ok(*returns),
                _ => todo!(),
            },

            // ast::Expr::Constructor(constructor) => {
            //     // TODO: get type from type-env
            //     // let a= Checked::Known(Type::Struct { ident: constructor.ident, fields: () })

            //     let Some(ty) = env.get(&constructor.ident) else {
            //         todo!("nice error message when struct does not exist {constructor:?}")
            //     };

            //     // TODO: i don't like having to clone here
            //     Ok(ty.clone())
            // }
            ast::Expr::Ref(inner) => Ok(Type::ptr(self.infer_expr(inner)?)),

            ast::Expr::Index(expr) => {
                let Type::List((inner_ty, _size)) = self.infer_expr(&expr.val)? else {
                    todo!("index val not a list");
                };
                Ok(*inner_ty)
            }

            // ast::Expr::If(_) | ast::Expr::RawIdent(_) => unimplemented!(),
            _ => todo!(),
        }
    }

    fn infer_stmnt(&self, stmnt: &ast::Stmnt) -> Result<Type> {
        match stmnt {
            _ => todo!(),
        }
    }

    pub fn lower(&mut self, node: ast::Node) -> Node {
        match node {
            ast::Node::Expr(expr) => Node::Expr(self.lower_expr(expr)),
            ast::Node::Stmnt(stmnt) => Node::Stmnt(self.lower_stmnt(stmnt)),
        }
    }

    pub fn lower_expr(&mut self, expr: ast::Expr) -> Expr {
        match expr {
            ast::Expr::IntLit(val) => Expr::IntLit(val),
            ast::Expr::StrLit(val) => Expr::StrLit(val),
            ast::Expr::Block(block) => {
                todo!()
            }
            // ast::Expr::
            _ => todo!(),
        }
    }

    pub fn lower_stmnt(&self, stmnt: ast::Stmnt) -> Stmnt {
        match stmnt {
            _ => todo!(),
        }
    }
}
