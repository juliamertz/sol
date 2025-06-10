use miette::{Diagnostic, Result};
use std::collections::HashMap;
use thiserror::Error;

use crate::ast::{self, Expr, Node, Stmnt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Int,
    Bool,
    Str,
    Fn {
        is_extern: bool,
        args: Vec<Type>,
        returns: Box<Type>,
    },
    List(Box<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked<T> {
    Known(T),
    Unknown,
}

impl From<&ast::Type> for Type {
    fn from(value: &ast::Type) -> Self {
        match value {
            ast::Type::Int => Self::Int,
            ast::Type::Bool => Self::Bool,
            ast::Type::Str => Self::Str,
            ast::Type::List(ty) => {
                let unboxed = Type::from(&(**ty)); // damn this is ugly
                Self::List(Box::new(unboxed))
            }
            _ => panic!("TODO: {value:?}"),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum AnalyzeError {
    #[error("Type mismatch between {lhs:?} and {rhs:?}")]
    TypeMismatch {
        lhs: Checked<Type>,
        rhs: Checked<Type>,
    },

    #[error("No such variable: '{name}'")]
    UndefinedVariable { name: String },
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    variables: HashMap<String, Checked<Type>>,
    functions: HashMap<String, Checked<Type>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn bind_var(&mut self, name: impl ToString, ty: Checked<Type>) {
        self.variables.insert(name.to_string(), ty);
    }

    pub fn lookup_var(&self, name: impl AsRef<str>) -> Option<&Checked<Type>> {
        self.variables.get(name.as_ref())
    }

    pub fn bind_fn(&mut self, name: impl ToString, ty: Checked<Type>) {
        self.functions.insert(name.to_string(), ty);
    }

    pub fn lookup_fn(&self, name: impl AsRef<str>) -> Option<&Checked<Type>> {
        self.functions.get(name.as_ref())
    }
}

pub struct Analyzer;

impl Analyzer {
    pub fn collect_declarations(nodes: &[Node], env: &mut TypeEnv) -> Result<()> {
        for node in nodes.iter() {
            match node {
                Node::Stmnt(Stmnt::Let(binding)) => {
                    let ty = match binding.ty {
                        Some(ref ty) => ty.into(),
                        None => Type::Unknown,
                    };

                    env.bind_var(&binding.ident, Checked::Known(ty.clone()));
                }

                Node::Stmnt(Stmnt::Fn(binding)) => {
                    let mut args = vec![];
                    for arg in binding.args.iter() {
                        args.push((&arg.ty).into());
                    }

                    let ty = Type::Fn {
                        args,
                        is_extern: binding.is_extern,
                        returns: Box::new((&binding.return_ty).into()),
                    };

                    env.bind_fn(binding.name.to_owned(), Checked::Known(ty));
                }

                _ => {}
            };
        }

        Ok(())
    }

    pub fn check_node(node: &Node, env: &mut TypeEnv) -> Result<Checked<Type>> {
        match node {
            Node::Expr(expr) => Self::check_expr(expr, env),
            Node::Stmnt(stmnt) => Self::check_stmnt(stmnt, env),
        }
    }

    fn check_expr(expr: &Expr, env: &mut TypeEnv) -> Result<Checked<Type>> {
        match expr {
            Expr::IntLit(_) => Ok(Checked::Known(Type::Int)),

            Expr::StringLit(_) => Ok(Checked::Known(Type::Str)),

            Expr::Infix(infix_expr) => {
                let lhs = Self::check_expr(&infix_expr.lhs, env)?;
                let rhs = Self::check_expr(&infix_expr.rhs, env)?;
                if lhs != rhs {
                    return Err(AnalyzeError::TypeMismatch { lhs, rhs }.into());
                }
                Ok(lhs)
            }

            Expr::List(list) => {
                let mut items = list.items.iter();
                let expected_ty = Self::check_expr(items.next().unwrap(), env)?;

                for expr in items {
                    let ty = Self::check_expr(expr, env)?;
                    if ty != expected_ty {
                        return Err(AnalyzeError::TypeMismatch {
                            lhs: ty,
                            rhs: expected_ty,
                        }
                        .into());
                    }
                }

                Ok(expected_ty)
            }

            Expr::Ident(name) => env
                .lookup_var(name)
                .cloned()
                .ok_or_else(|| AnalyzeError::UndefinedVariable { name: name.clone() }.into()),

            Expr::Call(_) | Expr::If(_) => unimplemented!(),
        }
    }

    fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv) -> Result<Checked<Type>> {
        match stmnt {
            Stmnt::Let(binding) => {
                let Some(ref expr) = binding.val else {
                    return Ok(Checked::Unknown);
                };
                // TODO: use type instead of inferring and then check it

                let ty = Self::check_expr(expr, env).unwrap();
                env.bind_var(&binding.ident, ty.clone());
                Ok(ty)
            }

            Stmnt::Fn(binding) => {
                let mut args: Vec<Type> = vec![];
                for arg in binding.args.iter() {
                    args.push((&arg.ty).into());
                }

                let ty = Type::Fn {
                    args,
                    is_extern: binding.is_extern,
                    returns: Box::new((&binding.return_ty).into()),
                };

                Ok(Checked::Known(ty))
            }

            Stmnt::Ret(_) => Ok(Checked::Unknown), // TODO:

            Stmnt::Use(_) => Ok(Checked::Unknown),
        }
    }
}
