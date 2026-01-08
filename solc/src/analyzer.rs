use miette::Diagnostic;
use solc_macros::Id;
use std::{collections::HashMap, hash::Hash};

use crate::ast::{
    BinOp, CallExpr, Constructor, Expr, Fn, Ident, IfElse, Impl, IndexExpr, Let, List, Literal,
    LiteralKind, Node, NodeId, OpKind, PrefixExpr, Ret, Stmnt, StructDef, Ty, TyKind, Use,
};

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum TypeError {
    #[error("variable not found in scope: {0}")]
    NotFound(Ident),
    #[error("type mismatch: {lhs:?} {rhs:?}")]
    TypeMismatch { lhs: Type, rhs: Type },
    #[error("invalid type, expected: {expected:?}, got: {actual:?}")]
    InvalidType { expected: Type, actual: Type },
}

pub type Result<T, E = TypeError> = core::result::Result<T, E>;

#[derive(Id, Debug, Clone, Copy)]
pub struct DefId(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    None,
    Int,
    Bool,
    Str,
    List((Box<Type>, Option<usize>)),
    Ptr(Box<Type>),
    Fn {
        is_extern: bool,
        params: Vec<Type>,
        returns: Box<Type>,
    },
    Struct {
        ident: Ident,
        fields: Vec<(Ident, Type)>,
    },
    Var(Ident),
}

impl From<&Ty> for Type {
    fn from(ty: &Ty) -> Self {
        Self::from(&ty.kind)
    }
}

impl From<&TyKind> for Type {
    fn from(kind: &TyKind) -> Self {
        match kind {
            TyKind::Int => Self::Int,
            TyKind::Bool => Self::Bool,
            TyKind::Str => Self::Str,
            TyKind::Var(name) => Self::Var(name.clone()),
            TyKind::List { inner, size } => {
                Self::List((Box::new(Self::from(inner.as_ref())), *size))
            }
            TyKind::Fn {
                params,
                returns,
                is_extern,
            } => Self::Fn {
                is_extern: *is_extern,
                params: params.iter().map(Self::from).collect(),
                returns: Box::new(Self::from(returns.as_ref())),
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    variables: HashMap<String, DefId>,
}

impl Scope<'_> {
    pub fn set_var(&mut self, ident: impl ToString, def_id: DefId) {
        self.variables.insert(ident.to_string(), def_id);
    }

    pub fn get_var(&self, ident: impl AsRef<str>) -> Option<&DefId> {
        self.variables.get(ident.as_ref()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_var(ident))
        })
    }

    pub fn child(&self) -> Scope<'_> {
        Scope {
            parent: Some(self),
            variables: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeEnv {
    def_idx: u32,
    pub nodes: HashMap<NodeId, Type>,
    pub definitions: HashMap<DefId, Type>,
}

impl TypeEnv {
    pub fn define(&mut self, ty: Type) -> DefId {
        let id = DefId(self.def_idx);
        self.def_idx += 1;
        self.definitions.insert(id, ty);
        id
    }

    pub fn get_definition(&self, id: &DefId) -> Option<&Type> {
        self.definitions.get(id)
    }

    pub fn set_type(&mut self, id: NodeId, ty: Type) {
        self.nodes.insert(id, ty);
    }

    pub fn type_of(&self, id: &NodeId) -> Option<&Type> {
        self.nodes.get(id)
    }
}

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<Type> {
    let ty = match expr {
        Expr::Ident(ident) => {
            let name = ident.inner.as_str();
            let def = scope
                .get_var(name)
                .ok_or(TypeError::NotFound(ident.to_owned()))?;
            let ty = env
                .get_definition(def)
                .expect("collected type for definition");
            Ok(ty.to_owned())
        }

        Expr::Literal(Literal { kind, .. }) => match kind {
            LiteralKind::Str(_) => Ok(Type::Str),
            LiteralKind::Int(_) => Ok(Type::Int),
        },

        Expr::Block(block) => {
            let scope = &mut scope.child();
            check_nodes(&block.nodes, env, scope)?;

            todo!()
        }

        Expr::BinOp(BinOp { lhs, op, rhs, .. }) => {
            let lhs = infer(lhs.as_ref(), env, scope)?;
            let rhs = infer(rhs.as_ref(), env, scope)?;

            match op.kind {
                OpKind::Eq | OpKind::Lt | OpKind::Gt => {
                    if lhs != rhs {
                        return Err(TypeError::TypeMismatch { lhs, rhs });
                    } else {
                        return Ok(Type::Bool);
                    }
                }

                OpKind::And | OpKind::Or => {
                    if lhs != Type::Bool {
                        todo!()
                    }
                    if rhs != Type::Bool {
                        todo!()
                    }

                    return Ok(Type::Bool);
                }

                _ => match (lhs, rhs) {
                    (Type::Int, Type::Int) => match op.kind {
                        OpKind::Add | OpKind::Sub | OpKind::Mul | OpKind::Div => Ok(Type::Int),
                        _ => todo!(),
                    },
                    _ => todo!(),
                },
            }
        }

        Expr::Prefix(PrefixExpr { op, rhs, .. }) => {
            let ty = infer(rhs, env, scope)?;
            match (&op.kind, ty) {
                (OpKind::Sub, Type::Int) => Ok(Type::Int),
                _ => todo!(),
            }
        }

        Expr::Call(CallExpr { func, params, .. }) => todo!("infer call expr"),

        Expr::Index(IndexExpr { expr, .. }) => {
            if let Type::List((ty, _)) = infer(expr, env, scope)? {
                Ok(ty.as_ref().to_owned())
            } else {
                todo!("can only index for list types")
            }
        }

        Expr::IfElse(IfElse {
            id,
            condition,
            consequence,
            alternative,
            ..
        }) => {
            let condition_ty = infer(&condition, env, scope)?;
            if condition_ty != Type::Bool {
                return Err(TypeError::InvalidType {
                    expected: Type::Bool,
                    actual: condition_ty,
                });
            }

            let block_scope = &mut scope.child();
            let consequence_ty = infer(&Expr::Block(consequence.to_owned()), env, block_scope)?;
            let alternative_ty = alternative
                .clone()
                .map(|alternative| infer(&Expr::Block(alternative), env, block_scope))
                .transpose()?;

            if let Some(alternative_ty) = alternative_ty
                && alternative_ty != consequence_ty
            {
                return Err(TypeError::TypeMismatch {
                    lhs: consequence_ty,
                    rhs: alternative_ty,
                });
            }

            env.set_type(*id, consequence_ty);

            Ok(todo!())
        }

        Expr::List(List { items, .. }) => {
            todo!()
        }

        Expr::Constructor(Constructor { ident, fields, .. }) => {
            let def_id = scope
                .get_var(ident)
                .ok_or(TypeError::NotFound(ident.to_owned()))?;
            let ty = env
                .get_definition(def_id)
                .expect("constructor type to be defined");
            Ok(ty.to_owned())
        }

        Expr::Ref(expr) => Ok(Type::Ptr(infer(expr, env, scope)?.into())),

        Expr::RawIdent(ident) => todo!("infer raw ident"),
    }?;

    env.set_type(expr.id(), ty.clone());

    Ok(ty)
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let { ident, ty, val, .. }) => {
            let ty = if let Some(ty) = ty {
                Type::from(ty)
            } else {
                infer(val, env, scope)?
            };

            let def_id = env.define(ty);
            scope.set_var(ident, def_id);
        }

        Stmnt::Ret(Ret { val, .. }) => todo!("check return statement"),

        Stmnt::Use(Use { ident, .. }) => {}

        Stmnt::Fn(Fn {
            is_extern,
            ident,
            params,
            return_ty,
            body,
            ..
        }) => {
            let ty = Type::Fn {
                is_extern: *is_extern,
                params: params.iter().map(|(_, ty)| ty.into()).collect(),
                returns: Box::new(return_ty.into()),
            };
            let def_id = env.define(ty);
            scope.set_var(ident, def_id);

            if let Some(body) = body
                && !is_extern
            {
                // FIX: no need for this clone
                let block = Expr::Block(body.to_owned());
                let ty = infer(&block, env, scope)?;
                dbg!(&ty);
            }
        }

        Stmnt::StructDef(StructDef { ident, fields, .. }) => {
            let ty = Type::Struct {
                ident: ident.to_owned(),
                fields: fields
                    .iter()
                    .map(|(ident, ty)| (ident.to_owned(), ty.into()))
                    .collect(),
            };
            let def_id = env.define(ty);
            scope.set_var(ident, def_id);
        }

        Stmnt::Impl(Impl { ident, body, .. }) => todo!("check impl block"),
    }

    Ok(())
}

pub fn check_node(node: &Node, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match node {
        Node::Expr(expr) => {
            let ty = infer(expr, env, scope)?;
            dbg!(&expr);
            env.set_type(expr.id(), ty);
            Ok(())
        }
        Node::Stmnt(stmnt) => check_stmnt(stmnt, env, scope),
    }
}

pub fn check_nodes(nodes: &[Node], env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    for node in nodes {
        check_node(node, env, scope)?;
    }
    Ok(())
}
