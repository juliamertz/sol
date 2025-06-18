use miette::{Diagnostic, Result};
use std::collections::HashMap;
use thiserror::Error;

use crate::ast::{self, Expr, Ident, Node, Stmnt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    Str,
    List(Box<Checked<Type>>),
    Ptr(Box<Checked<Type>>),
    Fn {
        is_extern: bool,
        args: Vec<Type>,
        returns: Box<Type>,
    },
    Struct {
        ident: Ident,
        fields: Vec<(Ident, Type)>,
    },
}

impl Type {
    fn list(ty: Checked<Type>) -> Self {
        Self::List(Box::new(ty))
    }

    fn checked(self) -> Checked<Type> {
        Checked::Known(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked<T> {
    Known(T),
    Unknown,
}

impl<T> Checked<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Known(value) => value,
            Self::Unknown => panic!("unwrap called on unknown"),
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    pub fn _is_known(&self) -> bool {
        !self.is_unknown()
    }
}

impl From<&ast::Type> for Type {
    fn from(value: &ast::Type) -> Self {
        match value {
            ast::Type::Int => Self::Int,
            ast::Type::Bool => Self::Bool,
            ast::Type::Str => Self::Str,
            ast::Type::List(ty) => {
                let unboxed = Type::from(&(**ty)); // damn this is ugly
                Self::list(Checked::Known(unboxed))
            }
            ast::Type::Struct { ident, fields } => todo!(),
            ast::Type::Fn {
                args,
                returns,
                is_extern,
            } => todo!(),
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
    definitions: HashMap<String, Checked<Type>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    pub fn bind(&mut self, name: impl ToString, ty: Checked<Type>) {
        self.definitions.insert(name.to_string(), ty);
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&Checked<Type>> {
        self.definitions.get(name.as_ref())
    }

    pub fn get_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Checked<Type>> {
        self.definitions.get_mut(name.as_ref())
    }
}

pub struct Analyzer;

impl Analyzer {
    pub fn collect_declarations(nodes: &[Node], env: &mut TypeEnv) -> Result<()> {
        for node in nodes.iter() {
            match node {
                Node::Stmnt(Stmnt::Let(binding)) => {
                    env.bind(
                        &binding.name,
                        match binding.ty {
                            Some(ref ty) => Checked::Known(ty.into()),
                            None => Checked::Unknown,
                        },
                    );
                }

                Node::Stmnt(Stmnt::Fn(binding)) => {
                    let mut args = vec![];
                    for (_, ty) in binding.args.iter() {
                        args.push(ty.into());
                    }

                    env.bind(
                        binding.name.to_owned(),
                        Checked::Known(Type::Fn {
                            args,
                            is_extern: binding.is_extern,
                            returns: Box::new((&binding.return_ty).into()),
                        }),
                    );
                }

                Node::Stmnt(Stmnt::StructDef(def)) => {
                    env.bind(
                        &def.ident,
                        Checked::Known(Type::Struct {
                            ident: def.ident.clone(),
                            fields: def
                                .fields
                                .iter()
                                .map(|(ident, ty)| (ident.clone(), ty.into()))
                                .collect(),
                        }),
                    );
                }

                _ => {}
            };
        }

        Ok(())
    }

    pub fn _check_node(node: &Node, env: &mut TypeEnv) -> Result<Checked<Type>> {
        match node {
            Node::Expr(expr) => Self::check_expr(expr, env),
            Node::Stmnt(stmnt) => Self::check_stmnt(stmnt, env),
        }
    }

    pub fn check_expr(expr: &Expr, env: &mut TypeEnv) -> Result<Checked<Type>> {
        match expr {
            Expr::IntLit(_) => Ok(Checked::Known(Type::Int)),

            Expr::StringLit(_) => Ok(Checked::Known(Type::Str)),

            Expr::Prefix(prefix_expr) => {
                todo!();
            },

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

                for item in items {
                    let ty = Self::check_expr(item, env)?;
                    if ty != expected_ty {
                        // TODO: add miette context that this is a list
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
                .get(name)
                .cloned()
                .ok_or_else(|| AnalyzeError::UndefinedVariable { name: name.clone() }.into()),

            Expr::Call(call_expr) => match Self::check_expr(&call_expr.func, env)? {
                Checked::Unknown => Ok(Checked::Unknown),
                Checked::Known(Type::Fn { returns, .. }) => Ok(Checked::Known(*returns)),
                _ => todo!(),
            },

            Expr::StructConstructor(constructor) => {
                // TODO: get type from type-env
                // let a= Checked::Known(Type::Struct { ident: constructor.ident, fields: () })

                let Some(ty) = env.get(&constructor.ident) else {
                    todo!("nice error message when struct does not exist {constructor:?}")
                };

                // TODO: i don't like having to clone here
                Ok(ty.clone())
            }

            Expr::If(_) => unimplemented!(),
        }
    }

    pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv) -> Result<Checked<Type>> {
        match stmnt {
            Stmnt::Let(binding) => {
                let value_ty = Analyzer::check_expr(binding.val.as_ref().unwrap(), env)?;
                dbg!(&value_ty);

                match env.get_mut(&binding.name) {
                    Some(Checked::Known(ty)) => {
                        if *ty != Type::list(value_ty.clone()) {
                            return Err(AnalyzeError::TypeMismatch {
                                lhs: ty.clone().checked(),
                                rhs: value_ty,
                            }
                            .into());
                        }
                    }

                    Some(checked) if checked.is_unknown() => {
                        *checked = value_ty;
                    }

                    _ => todo!(),
                }

                let Some(ref expr) = binding.val else {
                    return Ok(Checked::Unknown);
                };
                // TODO: use type instead of inferring and then check it

                let ty = Self::check_expr(expr, env).unwrap();
                env.bind(&binding.name, ty.clone());
                Ok(ty)
            }

            Stmnt::Fn(binding) => {
                let mut args: Vec<Type> = vec![];
                for (_, ty) in binding.args.iter() {
                    args.push((ty).into());
                }

                let ty = Type::Fn {
                    args,
                    is_extern: binding.is_extern,
                    returns: Box::new((&binding.return_ty).into()),
                };

                Ok(Checked::Known(ty))
            }

            Stmnt::StructDef(_) => Ok(Checked::Unknown),

            Stmnt::Ret(_) => Ok(Checked::Unknown), // TODO:

            Stmnt::Use(_) => Ok(Checked::Unknown),
        }
    }
}
