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
use crate::type_checker::interner::TypeInterner;

pub mod interner;
pub mod ty;

use ty::*;

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(solc::type_checker))]
pub enum TypeError {
    #[error("{ident} not found in scope")]
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

        lhs_ty: Type,
        #[label("has type `{lhs_ty}`")]
        lhs_span: Span,

        rhs_ty: Type,
        #[label("has type `{rhs_ty}`")]
        rhs_span: Span,

        #[help]
        help: Option<String>,
    },

    #[error("mismatched element types in list")]
    HeterogeneousList {
        #[source_code]
        src: SourceInfo,

        first_ty: Type,
        #[label("first element has type `{first_ty}`")]
        first_span: Span,

        other_ty: Type,
        #[label("this element has type `{other_ty}`")]
        other_span: Span,

        #[help]
        help: Option<String>,
    },

    #[error("internal type checker error")]
    Internal,
}

pub type Result<T, E = TypeError> = core::result::Result<T, E>;

id!(DefId);
id!(TypeId);

#[derive(Debug)]
pub struct Scope<'a> {
    src: SourceInfo,
    parent: Option<&'a Scope<'a>>,
    definitions: HashMap<Arc<str>, DefId>,
}

impl Scope<'_> {
    pub fn new(src: SourceInfo) -> Self {
        Self {
            src,
            parent: None,
            definitions: Default::default(),
        }
    }

    pub fn define(&mut self, ident: impl Into<Arc<str>>, def_id: DefId) {
        self.definitions.insert(ident.into(), def_id);
    }

    pub fn get_definition(&self, ident: &Ident) -> Option<&DefId> {
        self.definitions.get(ident.as_str()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_definition(ident))
        })
    }

    pub fn new_child(&self) -> Scope<'_> {
        Scope {
            src: self.src.clone(),
            parent: Some(self),
            definitions: Default::default(),
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

    pub fn type_from_ast_ty(&mut self, ty: &crate::ast::Ty, scope: &Scope<'_>) -> Result<TypeId> {
        let ty = match &ty.kind {
            crate::ast::TyKind::Int(kind) => Type::Int(kind.into()),
            crate::ast::TyKind::UInt(kind) => Type::UInt(kind.into()),
            crate::ast::TyKind::Bool => Type::Bool,
            crate::ast::TyKind::Str => Type::Str,
            crate::ast::TyKind::Var(ident) => {
                let def_id = scope.get_definition(ident).ok_or(TypeError::NotFound {
                    src: scope.src.clone(),
                    ident: ident.to_owned(),
                    span: ident.span,
                })?;
                let type_id = self.definitions.get(def_id).unwrap(); // TODO: handle error
                return Ok(*type_id);
            }
            crate::ast::TyKind::List { inner, size } => {
                let inner_id = self.type_from_ast_ty(inner, scope)?;
                Type::List(inner_id, *size)
            }
            crate::ast::TyKind::Fn {
                params,
                returns,
                is_extern,
            } => {
                let param_ids: Box<[TypeId]> = params
                    .iter()
                    .map(|param| self.type_from_ast_ty(param, scope))
                    .collect::<Result<Vec<_>>>()?
                    .into();
                let return_id = self.type_from_ast_ty(returns, scope)?;
                Type::Fn {
                    is_extern: *is_extern,
                    params: param_ids,
                    returns: return_id,
                }
            }
        };

        Ok(self.types.intern(ty))
    }
}

pub fn infer_ident(ident: &Ident, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    let def_id = scope
        .get_definition(ident)
        .ok_or_else(|| TypeError::NotFound {
            src: scope.src.clone(),
            ident: ident.to_owned(),
            span: ident.span,
        })?;
    let type_id = env
        .definitions
        .get(def_id)
        .copied()
        .ok_or(TypeError::Internal)?;
    Ok(type_id)
}

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    let ty = match expr {
        Expr::Ident(ident) => infer_ident(ident, env, scope),

        Expr::Literal(Literal { kind, .. }) => match kind {
            LiteralKind::Str(_) => Ok(env.types.intern(Type::Str)),
            LiteralKind::Int(_) => Ok(env.types.intern(Type::Int(ty::IntTy::I32))), // TODO: infer the correct size
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
                Ok(TypeId::NONE)
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
                        Ok(TypeId::BOOL)
                    }
                }

                OpKind::And | OpKind::Or => {
                    if lhs_ty != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: env.types.get(&lhs_ty).unwrap().clone(),
                            src: scope.src.clone(),
                            span: lhs.span(),
                        })
                    } else if rhs_ty != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: env.types.get(&rhs_ty).unwrap().clone(),
                            src: scope.src.clone(),
                            span: rhs.span(),
                        })
                    } else {
                        Ok(TypeId::BOOL)
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
            let func_type_id = infer(func, env, scope)?;
            let returns = {
                let func_ty = env.types.get(&func_type_id).unwrap();
                let Type::Fn { returns, .. } = func_ty else {
                    todo!("cannot call a non fn var");
                };
                *returns
            };

            for param in params.iter() {
                let _ty = infer(param, env, scope)?;
                // TODO: check validity of params
            }

            Ok(returns)
        }

        Expr::Index(IndexExpr { expr, .. }) => {
            let type_id = infer(expr, env, scope)?;
            let inner = {
                let ty = env.types.get(&type_id).unwrap();
                if let Type::List(inner, _) = ty {
                    *inner
                } else {
                    todo!("can only index for list types")
                }
            };
            Ok(inner)
        }

        Expr::IfElse(IfElse {
            condition,
            consequence,
            alternative,
            ..
        }) => {
            let condition_ty = infer(condition, env, scope)?;
            if condition_ty != TypeId::BOOL {
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

        Expr::List(List { items, .. }) => {
            let mut iter = items.iter();
            let first_item = iter.next();

            let inner_type = first_item
                .map(|expr| infer(expr, env, scope))
                .transpose()?
                .unwrap_or(TypeId::NONE);

            while let Some(item) = iter.next()
                && let Some(first_item) = first_item
            {
                let ty = infer(item, env, scope)?;
                if ty != inner_type {
                    return Err(TypeError::HeterogeneousList {
                        src: scope.src.clone(),
                        first_ty: env.types.get(&inner_type).unwrap().clone(),
                        first_span: first_item.span(),
                        other_ty: env.types.get(&ty).unwrap().clone(),
                        other_span: item.span(),
                        help: Some("pick a type and commit to it".into()),
                    });
                }
            }
            let ty = Type::List(inner_type, None); // TODO: fixed sized lists
            let type_id = env.types.intern(ty);
            Ok(type_id)
        }

        Expr::Constructor(Constructor { ident, fields, .. }) => {
            let def_id = scope
                .get_definition(ident)
                .ok_or_else(|| TypeError::NotFound {
                    src: scope.src.clone(),
                    ident: ident.to_owned(),
                    span: ident.span,
                })?;
            let type_id = *env
                .definitions
                .get(def_id)
                .expect("constructor type to be defined");

            for (_ident, expr) in fields.iter() {
                let _field_ty = infer(expr, env, scope)?;
                // TODO: validate fields
            }

            Ok(type_id)
        }

        Expr::MemberAccess(MemberAccess { lhs, ident, .. }) => {
            let lhs_type_id = infer(lhs, env, scope)?;
            let field_ty_id = {
                let lhs_ty = env.types.get(&lhs_type_id).unwrap();
                if let Type::Struct { fields, .. } = lhs_ty {
                    fields
                        .iter()
                        .find(|(field, _)| field.as_str() == ident.as_str())
                        .map(|(_, ty_id)| *ty_id)
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
            Ok(field_ty_id)
        }

        Expr::Ref(expr) => {
            let inner_type_id = infer(expr, env, scope)?;
            Ok(env.types.intern(Type::Ptr(inner_type_id)))
        }
    }?;

    env.nodes.insert(expr.id(), ty);

    Ok(ty)
}

pub fn infer_fn(func: &Fn, env: &mut TypeEnv, scope: &Scope<'_>) -> Result<Type> {
    Ok(Type::Fn {
        is_extern: func.is_extern,
        params: func
            .params
            .iter()
            .map(|(_, ty)| env.type_from_ast_ty(ty, scope))
            .collect::<Result<Vec<_>>>()?
            .into(),
        returns: env.type_from_ast_ty(&func.return_ty, scope)?,
    })
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let { ident, ty, val, .. }) => {
            let type_id = infer(val, env, scope)?;

            if let Some(declared_ty) = ty {
                let declared_type_id = env.type_from_ast_ty(declared_ty, scope)?;
                if declared_type_id != type_id {
                    return Err(TypeError::InvalidType {
                        src: scope.src.clone(),
                        span: val.span(),
                        expected: env.types.get(&declared_type_id).unwrap().clone(),
                        actual: env.types.get(&type_id).unwrap().clone(),
                    });
                }
            }

            let def_id = env.definitions.intern(type_id);
            scope.define(ident, def_id);
        }

        Stmnt::Ret(Ret { val, .. }) => {
            infer(val, env, scope)?;
        }

        Stmnt::Use(Use { ident: _, .. }) => {}

        Stmnt::Fn(func) => {
            let ty = infer_fn(func, env, scope)?;
            let type_id = env.types.intern(ty);
            let def_id = env.definitions.intern(type_id);
            scope.define(&func.ident, def_id);
        }

        Stmnt::StructDef(StructDef { ident, fields, .. }) => {
            let field_tys: Box<[(Ident, TypeId)]> = fields
                .iter()
                .map(|(ident, ty)| Ok((ident.to_owned(), env.type_from_ast_ty(ty, scope)?)))
                .collect::<Result<Vec<_>>>()?
                .into();
            let ty = Type::Struct {
                ident: ident.to_owned().boxed(),
                fields: field_tys,
            };
            let type_id = env.types.intern(ty);
            let def_id = env.definitions.intern(type_id);
            scope.define(ident, def_id);
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
