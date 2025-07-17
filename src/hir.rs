// a high level IR that closely maps to the AST
// but with type annotations and other useful information

use std::collections::HashMap;

use miette::{Context, Diagnostic, IntoDiagnostic, Result, SourceSpan, miette};
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
    #[error("Type mismatch expected: {0:?}, got: {1:?}")]
    TypeMismatch(Type, Type),

    #[error("No such variable: '{0}'")]
    UndefinedVariable(String),
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
    id: SymbolId,
    kind: SymbolKind,
    name: String,
    ty: Type,
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

#[derive(Debug, Default, Clone)]
struct TypeEnv {
    types: HashMap<String, SymbolId>,
    variables: HashMap<String, SymbolId>,
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

    // might be nice if we can just assert that a symbol must exist.
    fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id as usize)
    }

    fn infer(&self, node: &ast::Node, env: &mut TypeEnv) -> Result<Type> {
        match node {
            ast::Node::Expr(expr) => self.infer_expr(expr, env),
            ast::Node::Stmnt(stmnt) => self.infer_stmnt(stmnt, env),
        }
    }

    fn infer_expr(&self, expr: &ast::Expr, env: &mut TypeEnv) -> Result<Type> {
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
                let lhs = self.infer_expr(&infix_expr.lhs, env)?;
                let rhs = self.infer_expr(&infix_expr.rhs, env)?;
                if lhs != rhs {
                    return Err(TypeError::TypeMismatch(lhs, rhs))
                        .into_diagnostic()
                        .wrap_err("infix expression type mismatch");
                }
                Ok(lhs)
            }

            ast::Expr::List(list) => {
                let mut items = list.items.iter();
                let expected_ty = items
                    .next()
                    .map(|item| self.infer_expr(item, env))
                    .unwrap_or(Ok(Type::Any))?; // TODO: how do we handle inferring empty lists?

                for item in items {
                    let ty = self.infer_expr(item, env)?;
                    if ty != expected_ty {
                        return Err(TypeError::TypeMismatch(expected_ty, ty))
                            .into_diagnostic()
                            .wrap_err("List type mismatch");
                    }
                }

                Ok(Type::list(expected_ty, None)) // TODO: fixed size lists
            }

            ast::Expr::Ident(name) => env
                .variables
                .get(name)
                .map(|id| self.get_symbol(*id))
                .flatten()
                .ok_or(TypeError::UndefinedVariable(name.clone()).into())
                .map(|sym| sym.ty.clone()),

            ast::Expr::Constructor(constructor) => env
                .types
                .get(&constructor.ident)
                .map(|id| self.get_symbol(*id))
                .flatten()
                .ok_or(TypeError::UndefinedVariable(constructor.ident.clone()).into())
                .map(|sym| sym.ty.clone()),

            ast::Expr::Call(call_expr) => match self.infer_expr(&call_expr.func, env)? {
                Type::Fn { returns, .. } => Ok(*returns),
                _ => todo!(),
            },

            ast::Expr::Ref(inner) => Ok(Type::ptr(self.infer_expr(inner, env)?)),

            ast::Expr::Index(expr) => {
                let Type::List((inner_ty, _size)) = self.infer_expr(&expr.val, env)? else {
                    todo!("index val not a list");
                };
                Ok(*inner_ty)
            }

            // ast::Expr::If(_) | ast::Expr::RawIdent(_) => unimplemented!(),
            _ => todo!(),
        }
    }

    fn infer_stmnt(&self, stmnt: &ast::Stmnt, env: &mut TypeEnv) -> Result<Type> {
        match stmnt {
            ast::Stmnt::Let(binding) => {
                let value_ty = self.infer_expr(binding.val.as_ref().unwrap(), env)?;

                match env.get_mut(&binding.name) {
                    Some(ty) if !ty.is_concrete() => {
                        *ty = value_ty;
                    }

                    Some(known) => {
                        // FIX: incorrect
                        if *known != value_ty {
                            return Err(TypeError::TypeMismatch(known.clone(), value_ty).into());
                        }
                    }

                    _ => todo!(),
                }

                let Some(ref expr) = binding.val else {
                    return Ok(Type::Var(binding.name.clone()));
                };
                // TODO: use type instead of inferring and then check it

                let ty = self.infer_expr(expr, env).unwrap();
                env.bind(&binding.name, ty.clone());
                Ok(ty)
            }

            ast::Stmnt::Fn(binding) => {
                let mut args: Vec<Type> = vec![];
                for (_, ty) in binding.args.iter() {
                    args.push((ty).into());
                }

                let ty = Type::Fn {
                    args,
                    is_extern: binding.is_extern,
                    returns: Box::new((&binding.return_ty).into()),
                };

                Ok(ty)
            }

            ast::Stmnt::StructDef(def) => Ok(Type::Struct {
                ident: def.ident.clone(),
                fields: def
                    .fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), ty.into()))
                    .collect(),
            }),

            // Stmnt::Use(_) | Stmnt::Ret(_) | Stmnt::Impl(_) => Ok(Type::Any),
            _ => Ok(Type::Any),
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
