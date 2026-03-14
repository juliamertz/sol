// required for miette `Diagnostic` derive
// see: https://github.com/rust-lang/rust/issues/147648
#![allow(unused_assignments)]

use std::collections::HashMap;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::{
    BinOp, CallExpr, Constructor, Expr, Fn, Ident, IfElse, Impl, IndexExpr, Let, List, Literal,
    LiteralKind, MemberAccess, Node, NodeId, OpKind, PrefixExpr, Ret, Stmnt, StructDef, Use,
};
use crate::ext::Boxed;
use crate::id;
use crate::interner::Interner;
use crate::lexer::source::{SourceInfo, Span};
use crate::type_checker::interner::{TypeId, TypeInterner};

pub mod interner;
pub mod ty;

pub use ty::*;

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(solc::type_checker))]
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

id!(DefId);

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

    pub fn get_type(&self, ident: &Ident) -> Option<&DefId> {
        self.types.get(ident.as_str()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_type(ident))
        })
    }

    pub fn new_child(&self) -> Scope<'_> {
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
    pub types: Interner<TypeId, Type, TypeInterner>,
    pub definitions: Interner<DefId, TypeId>,
    pub nodes: Interner<NodeId, TypeId>,
}

impl TypeEnv {
    pub fn type_of(&self, node_id: &NodeId) -> Option<&Type> {
        self.nodes
            .get(node_id)
            .and_then(|type_id| self.types.get(type_id))
    }
}

fn resolve_type(ty_id: TypeId, env: &TypeEnv, scope: &Scope<'_>) -> TypeId {
    let ty = env.types.get(&ty_id).unwrap();
    if let Type::Var(ident) = ty {
        scope
            .get_type(ident)
            .and_then(|def_id| env.definitions.get(def_id))
            .copied()
            .unwrap_or(ty_id)
    } else {
        ty_id
    }
}

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
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
                .definitions
                .get(def)
                .expect("collected type for definition");
            Ok(*ty)
        }

        Expr::Literal(Literal { kind, .. }) => match kind {
            LiteralKind::Str(_) => Ok(env.types.intern(Type::Str)),
            LiteralKind::Int(_) => Ok(env.types.intern(Type::Int(ty::IntTy::I32.into()))), // TODO: infer the correct size
        },

        Expr::Block(block) => {
            let scope = &mut scope.new_child();
            check_nodes(&block.nodes, env, scope)?;

            if let Some(last_node) = block.nodes.last() {
                let return_ty = match last_node {
                    Node::Expr(expr) => env.nodes.get(&expr.id()).copied().unwrap(),
                    Node::Stmnt(Stmnt::Ret(Ret { val, .. })) => {
                        env.nodes.get(&val.id()).copied().unwrap()
                    }
                    _ => env.types.intern(Type::None),
                };
                Ok(return_ty)
            } else {
                Ok(*TypeId::NONE)
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
                            lhs_ty: env.types.get(&lhs_ty).unwrap().clone(),
                            rhs_span: rhs.span(),
                            rhs_ty: env.types.get(&rhs_ty).unwrap().clone(),
                            help: None,
                        })
                    } else {
                        Ok(*TypeId::BOOL)
                    }
                }

                OpKind::And | OpKind::Or => {
                    if lhs_ty != *TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: env.types.get(&lhs_ty).unwrap().clone(),
                            src: scope.src.clone(),
                            span: lhs.span(),
                        })
                    } else if rhs_ty != *TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: env.types.get(&rhs_ty).unwrap().clone(),
                            src: scope.src.clone(),
                            span: rhs.span(),
                        })
                    } else {
                        Ok(*TypeId::BOOL)
                    }
                }

                _ => {
                    let lhs_type = env.types.get(&lhs_ty).unwrap();
                    match lhs_type {
                        Type::Int(_) => match op.kind {
                            OpKind::Add | OpKind::Sub | OpKind::Mul | OpKind::Div => Ok(lhs_ty),
                            _ => todo!(),
                        },
                        _ => todo!(),
                    }
                }
            }
        }

        Expr::Prefix(PrefixExpr { op, rhs, .. }) => {
            let ty = infer(rhs, env, scope)?;
            match (&op.kind, env.types.get(&ty).unwrap()) {
                (OpKind::Sub, Type::Int(_)) => Ok(ty),
                _ => todo!(),
            }
        }

        Expr::Call(CallExpr { func, params, .. }) => {
            let func_ty_id = infer(func, env, scope)?;
            let returns = {
                let func_ty = env.types.get(&func_ty_id).unwrap();
                let Type::Fn { returns, .. } = func_ty else {
                    todo!("cannot call a non fn var");
                };
                returns.as_ref().clone()
            };

            for param in params.iter() {
                let _ty = infer(param, env, scope)?;
                // TODO: check validity of params
            }

            Ok(env.types.intern(returns))
        }

        Expr::Index(IndexExpr { expr, .. }) => {
            let ty_id = infer(expr, env, scope)?;
            let inner = {
                let ty = env.types.get(&ty_id).unwrap();
                if let Type::List((inner, _)) = ty {
                    inner.as_ref().clone()
                } else {
                    todo!("can only index for list types")
                }
            };
            Ok(env.types.intern(inner))
        }

        Expr::IfElse(IfElse {
            condition,
            consequence,
            alternative,
            ..
        }) => {
            let condition_ty = infer(condition, env, scope)?;
            if condition_ty != *TypeId::BOOL {
                return Err(TypeError::InvalidType {
                    src: scope.src.clone(),
                    span: condition.span(),
                    expected: Type::Bool,
                    actual: env.types.get(&condition_ty).unwrap().clone(),
                });
            }

            let block_scope = &mut scope.new_child();
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
                    lhs_ty: env.types.get(&consequence_ty).unwrap().clone(),
                    rhs_span: alternative.span,
                    rhs_ty: env.types.get(&alternative_ty).unwrap().clone(),
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
            let ty_id = *env
                .definitions
                .get(def_id)
                .expect("constructor type to be defined");

            for (_ident, expr) in fields.iter() {
                let _field_ty = infer(expr, env, scope)?;
                // TODO: validate fields
            }

            Ok(ty_id)
        }

        Expr::MemberAccess(MemberAccess { lhs, ident, .. }) => {
            let lhs_ty_id = infer(lhs, env, scope)?;
            let lhs_ty_id = resolve_type(lhs_ty_id, env, scope);
            let field_ty = {
                let lhs_ty = env.types.get(&lhs_ty_id).unwrap();
                if let Type::Struct { fields, .. } = lhs_ty {
                    fields
                        .iter()
                        .find(|(field, _)| field.as_str() == ident.as_str())
                        .map(|(_, ty)| ty.clone())
                        .ok_or_else(|| TypeError::NoSuchField {
                            src: scope.src.clone(),
                            ident: ident.clone(),
                            ty: lhs_ty.clone(),
                            span: lhs.span().enclosing_to(&ident.span),
                        })?
                } else {
                    todo!("infer member access expr")
                }
            };
            Ok(env.types.intern(field_ty))
        }

        Expr::Ref(expr) => {
            let inner_ty_id = infer(expr, env, scope)?;
            let inner_ty = env.types.get(&inner_ty_id).unwrap().clone();
            Ok(env.types.intern(Type::Ptr(Box::new(inner_ty))))
        }

        Expr::RawIdent(_ident) => todo!("infer raw ident"),
    }?;

    env.nodes.insert(expr.id(), ty);

    Ok(ty)
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let { ident, ty, val, .. }) => {
            let val_ty_id = infer(val, env, scope)?;
            let val_ty_id = resolve_type(val_ty_id, env, scope);

            if let Some(declared_ty) = ty.as_ref().map(Type::from) {
                let declared_ty_id = env.types.intern(declared_ty);
                let declared_ty_id = resolve_type(declared_ty_id, env, scope);
                if declared_ty_id != val_ty_id {
                    return Err(TypeError::InvalidType {
                        src: scope.src.clone(),
                        span: val.span(),
                        expected: env.types.get(&declared_ty_id).unwrap().clone(),
                        actual: env.types.get(&val_ty_id).unwrap().clone(),
                    });
                }
            }

            let def_id = env.definitions.intern(val_ty_id);
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
            let ty_id = env.types.intern(ty);
            let def_id = env.definitions.intern(ty_id);
            scope.set_var(ident, def_id);

            if let Some(body) = body
                && !is_extern
            {
                let block = Expr::Block(body.to_owned()); // FIX: no need for this clone
                let scope = &mut scope.new_child();
                for (ident, ty) in params.iter() {
                    let param_ty_id = env.types.intern(ty.into());
                    let def_id = env.definitions.intern(param_ty_id);
                    scope.set_var(ident, def_id);
                }

                infer(&block, env, scope)?;
            }
        }

        Stmnt::StructDef(StructDef { ident, fields, .. }) => {
            let ty = Type::Struct {
                ident: ident.to_owned().boxed(),
                fields: fields
                    .iter()
                    .map(|(ident, ty)| (ident.to_owned(), ty.into()))
                    .collect(),
            };
            let ty_id = env.types.intern(ty);
            let def_id = env.definitions.intern(ty_id);
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
            infer(expr, env, scope)?;
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
