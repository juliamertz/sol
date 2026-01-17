// required for miette `Diagnostic` derive
// see: https://github.com/rust-lang/rust/issues/147648
#![allow(unused_assignments)]

use std::collections::HashMap;
use std::sync::Arc;

use miette::Diagnostic;
use solc_macros::Id;
use thiserror::Error;

use crate::lexer::source::{SourceInfo, Span};
use crate::ast::{
    BinOp, CallExpr, Constructor, Expr, Fn, Ident, IfElse, Impl, IndexExpr, Let, List, Literal,
    LiteralKind, MemberAccess, Node, NodeId, OpKind, PrefixExpr, Ret, Stmnt, StructDef, Use,
};

pub mod ty;
pub use ty::*;

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(analyzer))]
pub enum TypeError {
    #[error("variable not found in scope")]
    NotFound {
        #[source_code]
        src: SourceInfo,

        ident: Ident,

        #[label("this variable here")]
        span: Span,
    },

    #[error("no field '{ident}' on type: '{ty}'")]
    NoSuchField {
        #[source_code]
        src: SourceInfo,

        ident: Ident,

        ty: Type,

        #[label("here")]
        span: Span,
    },

    #[error("invalid type, expected: {expected:?}, got: {actual:?}")]
    InvalidType {
        expected: Type,
        actual: Type,

        #[source_code]
        src: SourceInfo,

        #[label("here")]
        span: Span,
    },

    #[error("mismatched types in comparison")]
    ComparisonMismatch {
        #[source_code]
        src: SourceInfo,

        #[label("has type `{lhs_ty}`")]
        lhs_span: Span,
        lhs_ty: Type,

        #[label("has type `{rhs_ty}`")]
        rhs_span: Span,
        rhs_ty: Type,

        #[help]
        help: Option<String>,
    },
}

pub type Result<T, E = TypeError> = core::result::Result<T, E>;

#[derive(Id, Debug, Clone, Copy)]
pub struct DefId(u32);

#[derive(Debug)]
pub struct Scope<'a> {
    src: SourceInfo,
    parent: Option<&'a Scope<'a>>,
    variables: HashMap<Arc<str>, DefId>,
    types: HashMap<Arc<str>, DefId>,
}

impl Scope<'_> {
    pub fn new(src: SourceInfo) -> Self {
        Self {
            src,
            parent: None,
            variables: Default::default(),
            types: Default::default(),
        }
    }

    pub fn set_var(&mut self, ident: impl Into<Arc<str>>, def_id: DefId) {
        self.variables.insert(ident.into(), def_id);
    }

    pub fn get_var(&self, ident: impl AsRef<str>) -> Option<&DefId> {
        self.variables.get(ident.as_ref()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_var(ident))
        })
    }

    pub fn set_type(&mut self, ident: impl Into<Arc<str>>, def_id: DefId) {
        self.types.insert(ident.into(), def_id);
    }

    pub fn get_type(&self, ident: impl AsRef<str>) -> Option<&DefId> {
        self.types.get(ident.as_ref()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_type(ident))
        })
    }

    pub fn child(&self) -> Scope<'_> {
        Scope {
            src: self.src.clone(),
            parent: Some(self),
            variables: Default::default(),
            types: Default::default(),
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
            let def = scope
                .get_var(&ident.inner)
                .ok_or_else(|| TypeError::NotFound {
                    src: scope.src.clone(),
                    ident: ident.to_owned(),
                    span: ident.span,
                })?;
            let ty = env
                .get_definition(def)
                .expect("collected type for definition");
            Ok(ty.to_owned())
        }

        Expr::Literal(Literal { kind, .. }) => match kind {
            LiteralKind::Str(_) => Ok(Type::Str),
            LiteralKind::Int(_) => Ok(Type::Int(ty::IntTy::I32.into())), // TODO: infer the correct size
        },

        Expr::Block(block) => {
            let scope = &mut scope.child();
            check_nodes(&block.nodes, env, scope)?;

            if let Some(last_node) = block.nodes.last() {
                let return_ty = match last_node {
                    Node::Expr(expr) => env.type_of(&expr.id()).unwrap().clone(),
                    Node::Stmnt(Stmnt::Ret(Ret { val, .. })) => {
                        env.type_of(&val.id()).unwrap().clone()
                    }
                    _ => Type::None,
                };
                Ok(return_ty)
            } else {
                Ok(Type::None)
            }
        }

        Expr::BinOp(BinOp { lhs, op, rhs, .. }) => {
            let lhs_ty = infer(lhs.as_ref(), env, scope)?;
            let rhs_ty = infer(rhs.as_ref(), env, scope)?;

            match op.kind {
                OpKind::Eq | OpKind::Lt | OpKind::Gt => {
                    if lhs_ty != rhs_ty {
                        Err(TypeError::ComparisonMismatch {
                            src: scope.src.clone(),
                            lhs_span: lhs.span(),
                            lhs_ty,
                            rhs_span: rhs.span(),
                            rhs_ty,
                            help: None,
                        })
                    } else {
                        Ok(Type::Bool)
                    }
                }

                OpKind::And | OpKind::Or => {
                    if lhs_ty != Type::Bool {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: lhs_ty,
                            src: scope.src.clone(),
                            span: lhs.span(),
                        })
                    } else if rhs_ty != Type::Bool {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: rhs_ty,
                            src: scope.src.clone(),
                            span: rhs.span(),
                        })
                    } else {
                        Ok(Type::Bool)
                    }
                }

                _ => match (lhs_ty, rhs_ty) {
                    (Type::Int(lhs_kind), Type::Int(_rhs_kind)) => match op.kind {
                        OpKind::Add | OpKind::Sub | OpKind::Mul | OpKind::Div => {
                            Ok(Type::Int(lhs_kind))
                        }
                        _ => todo!(),
                    },
                    _ => todo!(),
                },
            }
        }

        Expr::Prefix(PrefixExpr { op, rhs, .. }) => {
            let ty = infer(rhs, env, scope)?;
            match (&op.kind, ty) {
                (OpKind::Sub, Type::Int(kind)) => Ok(Type::Int(kind)),
                _ => todo!(),
            }
        }

        Expr::Call(CallExpr { func, params, .. }) => {
            let Type::Fn {
                is_extern: _,
                params: _param_types,
                returns,
            } = infer(func, env, scope)?
            else {
                todo!("cannot call a non fn var");
            };

            for param in params.iter() {
                let _ty = infer(param, env, scope)?;
                // TODO:
            }

            // TODO: check validity of params

            Ok(returns.as_ref().to_owned())
        }

        Expr::Index(IndexExpr { expr, .. }) => {
            if let Type::List((ty, _)) = infer(expr, env, scope)? {
                Ok(ty.as_ref().to_owned())
            } else {
                todo!("can only index for list types")
            }
        }

        Expr::IfElse(IfElse {
            condition,
            consequence,
            alternative,
            ..
        }) => {
            let condition_ty = infer(condition, env, scope)?;
            if condition_ty != Type::Bool {
                return Err(TypeError::InvalidType {
                    src: scope.src.clone(),
                    span: condition.span(),
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
                && let Some(alternative) = alternative
                && alternative_ty != consequence_ty
            {
                return Err(TypeError::ComparisonMismatch {
                    src: scope.src.clone(),
                    lhs_span: consequence.span,
                    lhs_ty: consequence_ty,
                    rhs_span: alternative.span,
                    rhs_ty: alternative_ty,
                    help: None,
                });
            }

            Ok(consequence_ty)
        }

        Expr::List(List { items: _, .. }) => {
            todo!()
        }

        Expr::Constructor(Constructor { ident, fields, .. }) => {
            let def_id = scope.get_type(ident).ok_or_else(|| TypeError::NotFound {
                src: scope.src.clone(),
                ident: ident.to_owned(),
                span: ident.span,
            })?;
            let ty = env
                .get_definition(def_id)
                .expect("constructor type to be defined")
                .clone();

            for (ident, expr) in fields.iter() {
                dbg!(&ident, &expr);
                let field_ty = infer(expr, env, scope)?;
                dbg!(&field_ty);
            }

            Ok(ty.to_owned())
        }

        Expr::MemberAccess(MemberAccess { lhs, ident, .. }) => {
            let lhs_ty = infer(lhs, env, scope)?.resolved(env, scope);

            if let Type::Struct { ref fields, .. } = lhs_ty {
                fields
                    .iter()
                    .find(|(field, _)| field.as_ref() == ident.as_ref())
                    .map(|(_, ty)| ty)
                    .ok_or_else(|| TypeError::NoSuchField {
                        src: scope.src.clone(),
                        ident: ident.clone(),
                        ty: lhs_ty.clone(),
                        span: lhs.span().enclosing_to(&ident.span),
                    })
                    .cloned()
            } else {
                todo!("infer member access expr")
            }
        }

        Expr::Ref(expr) => Ok(Type::Ptr(infer(expr, env, scope)?.into())),

        Expr::RawIdent(_ident) => todo!("infer raw ident"),
    }?;

    env.set_type(expr.id(), ty.clone());

    Ok(ty)
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let { ident, ty, val, .. }) => {
            let val_ty = infer(val, env, scope)?.resolved(env, scope);
            if let Some(ty) = ty
                .as_ref()
                .map(Type::from)
                .map(|ty| ty.resolved(env, scope))
                && ty != val_ty
            {
                return Err(TypeError::InvalidType {
                    src: scope.src.clone(),
                    span: val.span(),
                    expected: ty,
                    actual: val_ty,
                });
            };

            let def_id = env.define(val_ty);
            scope.set_var(ident, def_id);
        }

        Stmnt::Ret(Ret { val, .. }) => {
            infer(val, env, scope)?;
        }

        Stmnt::Use(Use { ident: _, .. }) => {}

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

            let _body_ty = if let Some(body) = body
                && !is_extern
            {
                let block = Expr::Block(body.to_owned()); // FIX: no need for this clone
                let scope = &mut scope.child();
                for (ident, ty) in params.iter() {
                    let def_id = env.define(ty.into());
                    scope.set_var(ident, def_id);
                }

                infer(&block, env, scope)?
            } else {
                Type::None
            };
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
            scope.set_type(ident, def_id);
        }

        Stmnt::Impl(Impl {
            ident: _, body: _, ..
        }) => todo!("check impl block"),
    }

    Ok(())
}

pub fn check_node(node: &Node, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match node {
        Node::Expr(expr) => {
            let ty = infer(expr, env, scope)?;
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
