use miette::{Diagnostic, Result};
use std::collections::HashMap;
use thiserror::Error;

use crate::ast::{self, Expr, Ident, Node, Stmnt};

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
        ident: Ident,
        fields: Vec<(Ident, Type)>,
    },
    // TODO: include sourcespan so we can have nicer debug messages
    Var(Ident),
}

impl Type {
    fn list(ty: Type, size: Option<usize>) -> Self {
        Self::List((Box::new(ty), size))
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
            ast::Type::List((ty, size)) => {
                let unboxed = Type::from(&(**ty)); // damn this is ugly
                Self::list(unboxed, *size)
            }
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
pub enum AnalyzeError {
    #[error("Type mismatch between {lhs:?} and {rhs:?}")]
    TypeMismatch { lhs: Type, rhs: Type },

    #[error("No such variable: '{name}'")]
    UndefinedVariable { name: String },
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    definitions: HashMap<String, Type>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    pub fn bind(&mut self, name: impl ToString, ty: Type) {
        self.definitions.insert(name.to_string(), ty);
    }

    pub fn get(&self, name: impl AsRef<str>) -> Option<&Type> {
        self.definitions.get(name.as_ref())
    }

    pub fn get_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Type> {
        self.definitions.get_mut(name.as_ref())
    }

    pub fn extend(&mut self, declarations: Vec<(&String, Type)>) {
        for (ident, ty) in declarations {
            // TODO: lol we have to rename one of these
            self.definitions.insert(ident.to_string(), ty);
        }
    }
}

pub struct Analyzer;

impl Analyzer {
    /// put new declarations in the typeenv env
    /// and return a list of items that should be pre-defined such as lists or strings
    pub fn collect_declarations(nodes: &[Node], env: &mut TypeEnv) -> Result<Vec<(String, Type)>> {
        // There must be a cleaner way to do this.
        // TODO: cleanup
        let mut predefine = vec![];

        for node in nodes {
            let def = match node {
                Node::Stmnt(Stmnt::Let(binding)) => {
                    let ty = match binding.ty {
                        Some(ref ty) => ty.into(),
                        // TODO: check binding type
                        None => Self::check_expr(binding.val.as_ref().unwrap(), env).unwrap(),
                    };

                    // TODO: this is a bit ugly
                    if matches!(ty, Type::List(_)) {
                        predefine.push((binding.name.clone(), ty.clone()))
                    }

                    Some((&binding.name, ty))
                }

                Node::Stmnt(Stmnt::Fn(binding)) => {
                    let mut args = vec![];
                    for (_, ty) in binding.args.iter() {
                        args.push(ty.into());
                    }

                    let ty = Type::Fn {
                        args,
                        is_extern: binding.is_extern,
                        returns: Box::new((&binding.return_ty).into()),
                    };

                    Some((&binding.name, ty))
                }

                Node::Stmnt(Stmnt::StructDef(def)) => {
                    let ty = Type::Struct {
                        ident: def.ident.clone(),
                        fields: def
                            .fields
                            .iter()
                            .map(|(ident, ty)| (ident.clone(), ty.into()))
                            .collect(),
                    };

                    Some((&def.ident, ty))
                }

                _ => None,
            };

            let Some((name, ty)) = def else {
                continue;
            };

            env.bind(name, ty);
        }
        Ok(predefine)
    }

    pub fn _check_node(node: &Node, env: &mut TypeEnv) -> Result<Type> {
        match node {
            Node::Expr(expr) => Self::check_expr(expr, env),
            Node::Stmnt(stmnt) => Self::check_stmnt(stmnt, env),
        }
    }

    pub fn check_expr(expr: &Expr, env: &mut TypeEnv) -> Result<Type> {
        match expr {
            Expr::IntLit(_) => Ok(Type::Int),

            Expr::Block(block) => {
                todo!("check block expressions")
            }

            Expr::StrLit(_) => Ok(Type::Str),

            Expr::Prefix(prefix_expr) => {
                todo!();
            }

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

                Ok(Type::list(expected_ty, None))
            }

            Expr::Ident(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| AnalyzeError::UndefinedVariable { name: name.clone() }.into()),

            Expr::Call(call_expr) => match Self::check_expr(&call_expr.func, env)? {
                Type::Fn { returns, .. } => Ok(*returns),
                _ => todo!(),
            },

            Expr::Constructor(constructor) => {
                // TODO: get type from type-env
                // let a= Checked::Known(Type::Struct { ident: constructor.ident, fields: () })

                let Some(ty) = env.get(&constructor.ident) else {
                    todo!("nice error message when struct does not exist {constructor:?}")
                };

                // TODO: i don't like having to clone here
                Ok(ty.clone())
            }

            Expr::Ref(inner) => Ok(Type::Ptr(Box::new(Self::check_expr(inner, env)?))),

            Expr::Index(expr) => {
                let Type::List((inner_ty, _size)) = Self::check_expr(&expr.val, env)? else {
                    todo!("index val not a list");
                };
                Ok(*inner_ty)
            }

            Expr::If(_) | Expr::RawIdent(_) => unimplemented!(),
        }
    }

    pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv) -> Result<Type> {
        match stmnt {
            Stmnt::Let(binding) => {
                let value_ty = Analyzer::check_expr(binding.val.as_ref().unwrap(), env)?;

                match env.get_mut(&binding.name) {
                    Some(ty) if !ty.is_concrete() => {
                        *ty = value_ty;
                    }

                    Some(known) => {
                        // FIX: incorrect
                        if *known != value_ty {
                            return Err(AnalyzeError::TypeMismatch {
                                lhs: known.clone(),
                                rhs: value_ty,
                            }
                            .into());
                        }
                    }

                    _ => todo!(),
                }

                let Some(ref expr) = binding.val else {
                    return Ok(Type::Var(binding.name.clone()));
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

                Ok(ty)
            }

            Stmnt::StructDef(def) => Ok(Type::Struct {
                ident: def.ident.clone(),
                fields: def
                    .fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), ty.into()))
                    .collect(),
            }),

            Stmnt::Use(_) | Stmnt::Ret(_) | Stmnt::Impl(_) => Ok(Type::Any),
        }
    }
}
