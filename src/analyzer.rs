use std::collections::HashMap;
use std::str::FromStr;

use crate::ast::*;
use crate::lexer::{Token, TokenKind};

use miette::{Diagnostic, NamedSource, Result, SourceSpan, miette};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Int,
    Bool,
    Str,
    Fn { r#extern: bool, args: Vec<Type>, returns: Box<Type> },
    List(Box<Type>),
}

// TODO: do this in parser first, then convert ast type to analyzer type
impl FromStr for Type {
    type Err = miette::Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Int" => Ok(Self::Int),
            "Bool" => Ok(Self::Bool),
            "Str" => Ok(Self::Str),
            _ => Err(miette!("cannot parse type from '{s}'")),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum AnalyzeError {
    #[error("Type mismatch between {lhs:?} and {rhs:?}")]
    TypeMismatch { lhs: Type, rhs: Type },
}

#[derive(Debug, Clone)]
struct TypeEnv {
    variables: HashMap<String, Type>,
    functions: HashMap<String, Type>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn bind(&mut self, name: impl ToString, ty: Type) {
        self.variables.insert(name.to_string(), ty);
    }

    pub fn lookup(&self, name: impl AsRef<str>) -> Option<&Type> {
        self.variables.get(name.as_ref())
    }

    pub fn extend(&self, name: impl ToString, ty: Type) -> Self {
        let mut new_env = self.clone();
        new_env.bind(name.to_string(), ty);
        new_env
    }
}

pub struct Analyzer {
    ast: Vec<Node>,
    root: TypeEnv,
}

impl Analyzer {
    pub fn new(ast: Vec<Node>) -> Self {
        Self {
            ast,
            root: TypeEnv::new(),
        }
    }

    pub fn check_node(&self, node: &Node, env: &mut TypeEnv) -> Result<Type> {
        match node {
            Node::Expr(expr) => self.check_expr(expr, env),
            Node::Stmnt(stmnt) => self.check_stmnt(stmnt, env),
        }
    }

    fn check_expr(&self, expr: &Expr, env: &mut TypeEnv) -> Result<Type> {
        match expr {
            Expr::IntLit(_) => Ok(Type::Int),
            Expr::StringLit(_) => Ok(Type::Str),
            Expr::Infix(infix_expr) => {
                let lhs = self.check_expr(&infix_expr.lhs, env)?;
                let rhs = self.check_expr(&infix_expr.rhs, env)?;
                if lhs != rhs {
                    return Err(AnalyzeError::TypeMismatch { lhs, rhs }.into());
                }
                Ok(lhs)
            },
            Expr::List(list) => {
                let mut items = list.items.iter();
                let expected_ty = self.check_expr(items.next().unwrap(), env)?;

                while let Some(expr) = items.next() {
                   let ty = self.check_expr(expr, env)?;
                   if ty != expected_ty {
                       return Err(AnalyzeError::TypeMismatch { lhs: ty, rhs: expected_ty }.into());
                   }
                }

                Ok(Type::List(Box::new(expected_ty)))
            }
            _ => todo!(),
        }
    }

    fn check_stmnt(&self, stmnt: &Stmnt, env: &mut TypeEnv) -> Result<Type> {
        match stmnt {
            Stmnt::Let(binding) => {
                let Some(ref expr) = binding.val else {
                    return Ok(Type::Unknown);
                };
                // TODO: use type instead of inferring and then check it

                let ty = self.check_expr(&expr, env).unwrap();
                env.bind(&binding.ident, ty.clone());
                Ok(ty)
            }

            Stmnt::Fn(binding) => {
                let mut args = vec![];
                for arg in binding.args.iter() {
                    let ty = Type::from_str(&arg.ty)?;
                    args.push(ty);
                }

                let returns = Type::from_str(&binding.return_ty)?;

                Ok(Type::Fn {
                    args,
                    r#extern: binding.r#extern,
                    returns: Box::new(returns),
                })
            }

            Stmnt::Ret(_) => Ok(Type::Unknown), // TODO:

            Stmnt::Use(_) => Ok(Type::Unknown),
        }
    }
}
