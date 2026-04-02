use std::collections::HashMap;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::{
    self, BinOp, BinOpKind, Block, CallExpr, Constructor, Expr, Fn, Ident, IfElse, Impl, IndexExpr,
    Item, Let, List, Literal, LiteralKind, MemberAccess, Module, NodeId, Ret, Stmnt, Unary,
    UnaryOpKind,
};
use crate::ext::{AsStr, Boxed};
use crate::id;
use crate::interner::Interner;
use crate::lexer::source::{SourceInfo, Span};
use crate::type_checker::collect::{CollectError, collect};
use crate::type_checker::interner::TypeInterner;
use crate::type_checker::ty::*;

pub mod collect;
pub mod interner;
pub mod ty;

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(solc::type_checker))]
pub enum TypeError {
    #[diagnostic(forward(0))]
    #[error(transparent)]
    Collect(#[from] CollectError),

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

    #[error("type not found for node")]
    UntypedNode {
        node_id: NodeId,

        #[source_code]
        src: SourceInfo,

        #[label("this node")]
        span: Span,
    },

    #[error("internal type checker error")]
    Internal,
}

pub type Result<T, E = TypeError> = core::result::Result<T, E>;

id!(DefId);
id!(TypeId);

#[derive(Debug, Default)]
pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    definitions: HashMap<Arc<str>, DefId>,
}

impl Scope<'_> {
    pub fn define(&mut self, name: impl AsStr, def_id: DefId) {
        self.definitions.insert(name.as_str().into(), def_id);
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
            parent: Some(self),
            definitions: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct TypeEnv {
    pub src: SourceInfo,
    pub types: Interner<TypeId, Type, TypeInterner>,
    pub definitions: Interner<DefId, TypeId>,
    pub nodes: Interner<NodeId, TypeId>,
    pub node_defs: HashMap<NodeId, DefId>,
    pub def_names: HashMap<DefId, Arc<str>>,
}

impl TypeEnv {
    pub fn new(src: SourceInfo) -> Self {
        Self {
            src,
            types: Default::default(),
            definitions: Default::default(),
            nodes: Default::default(),
            node_defs: Default::default(),
            def_names: Default::default(),
        }
    }

    pub fn type_of(&self, node_id: &NodeId, span: &Span) -> Result<TypeId> {
        self.nodes
            .get(node_id)
            .copied()
            .ok_or_else(|| TypeError::UntypedNode {
                node_id: *node_id,
                src: self.src.clone(),
                span: *span,
            })
    }

    pub fn type_by_id(&self, type_id: &TypeId) -> Result<&Type> {
        Ok(self.types.get(type_id).unwrap())
    }

    pub fn type_from_ast_ty(&mut self, ast_ty: &ast::Ty, scope: &Scope<'_>) -> Result<TypeId> {
        let ty = match &ast_ty.kind {
            ast::TyKind::Int(kind) => Type::Int(kind.into()),
            ast::TyKind::UInt(kind) => Type::UInt(kind.into()),
            ast::TyKind::Bool => Type::Bool,
            ast::TyKind::Str => Type::Str,
            ast::TyKind::Var(ident) => {
                let def_id = scope.get_definition(ident).ok_or(TypeError::NotFound {
                    src: self.src.clone(),
                    ident: ident.to_owned(),
                    span: ident.span,
                })?;
                let ty_id = self.definitions.get(def_id).copied().unwrap(); // TODO: handle error
                self.nodes.insert(ast_ty.id, ty_id);
                return Ok(ty_id);
            }
            ast::TyKind::List { inner, size } => {
                let inner_id = self.type_from_ast_ty(inner, scope)?;
                Type::List(inner_id, *size)
            }
            ast::TyKind::Fn {
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
                    is_variadic: false, // FIXME:????????
                    params: param_ids,
                    returns: return_id,
                }
            }
        };

        let ty_id = self.types.intern(ty);
        self.nodes.insert(ast_ty.id, ty_id);
        Ok(ty_id)
    }
}

pub fn infer_ident(ident: &Ident, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    let def_id = scope
        .get_definition(ident)
        .ok_or_else(|| TypeError::NotFound {
            src: env.src.clone(),
            ident: ident.to_owned(),
            span: ident.span,
        })?;
    env.node_defs.insert(ident.id, *def_id);
    let ty_id = env
        .definitions
        .get(def_id)
        .copied()
        .ok_or(TypeError::Internal)?;
    Ok(ty_id)
}

pub fn infer_block(block: &Block, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    check_stmnts(&block.nodes, env, scope)?;

    let ty_id = if let Some(last) = block.nodes.last() {
        match last {
            Stmnt::Expr(expr) => env.nodes.get(&expr.id()).copied().unwrap(),
            Stmnt::Ret(Ret { val, .. }) => env.nodes.get(&val.id()).copied().unwrap(),
            _ => env.types.intern(Type::Unit),
        }
    } else {
        TypeId::NONE
    };

    env.nodes.insert(block.id, ty_id);

    Ok(ty_id)
}

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    let ty = match expr {
        Expr::Ident(ident) => infer_ident(ident, env, scope),

        Expr::Literal(Literal { id, kind, .. }) => match kind {
            LiteralKind::Str(_) => {
                let ty_id = TypeId::STR;
                env.nodes.insert(*id, ty_id);
                Ok(ty_id)
            }
            LiteralKind::Int(_) => {
                let ty_id = TypeId::I32; // TODO: infer the correct size
                env.nodes.insert(*id, ty_id);
                Ok(ty_id)
            }
            LiteralKind::Bool(_) => {
                let ty_id = TypeId::BOOL;
                env.nodes.insert(*id, ty_id);
                Ok(ty_id)
            }
        },

        Expr::Block(block) => {
            let scope = &mut scope.new_child();
            infer_block(block, env, scope)
        }

        Expr::BinOp(BinOp { lhs, op, rhs, .. }) => {
            let lhs_ty = infer(lhs.as_ref(), env, scope)?;
            let rhs_ty = infer(rhs.as_ref(), env, scope)?;

            match op.kind {
                BinOpKind::Eq | BinOpKind::Lt | BinOpKind::Gt => {
                    if lhs_ty != rhs_ty {
                        Err(TypeError::ComparisonMismatch {
                            src: env.src.clone(),
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

                BinOpKind::And | BinOpKind::Or => {
                    if lhs_ty != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: env.types.get(&lhs_ty).unwrap().clone(),
                            src: env.src.clone(),
                            span: lhs.span(),
                        })
                    } else if rhs_ty != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Type::Bool,
                            actual: env.types.get(&rhs_ty).unwrap().clone(),
                            src: env.src.clone(),
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
                            BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div => {
                                Ok(lhs_ty)
                            }
                            _ => todo!(),
                        },
                        _ => todo!(),
                    }
                }
            }
        }

        Expr::Unary(Unary { op, rhs, .. }) => {
            let ty = infer(rhs, env, scope)?;
            match (&op.kind, env.types.get(&ty).unwrap()) {
                (UnaryOpKind::Negate, Type::Int(_)) => Ok(ty),
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
                *returns
            };

            for param in params.iter() {
                let _ty = infer(param, env, scope)?;
                // TODO: check validity of params
            }

            Ok(returns)
        }

        Expr::Index(IndexExpr { id, expr, idx, .. }) => {
            let val_ty_id = infer(expr, env, scope)?;
            env.nodes.insert(expr.id(), val_ty_id);

            let idx_ty_id = infer(idx, env, scope)?;
            env.nodes.insert(idx.id(), idx_ty_id);

            let inner = {
                let ty = env.types.get(&val_ty_id).unwrap();
                if let Type::List(inner, _) = ty {
                    *inner
                } else {
                    todo!("can only index for list types")
                }
            };

            env.nodes.insert(*id, inner);
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
                    src: env.src.clone(),
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
                    src: env.src.clone(),
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
            let size = items.len();
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
                        src: env.src.clone(),
                        first_ty: env.types.get(&inner_type).unwrap().clone(),
                        first_span: first_item.span(),
                        other_ty: env.types.get(&ty).unwrap().clone(),
                        other_span: item.span(),
                        help: Some("pick a type and commit to it".into()),
                    });
                }
            }

            let ty = Type::List(inner_type, Some(size));
            let ty_id = env.types.intern(ty);
            Ok(ty_id)
        }

        Expr::Constructor(Constructor {
            id, ident, fields, ..
        }) => {
            let def_id = scope
                .get_definition(ident)
                .ok_or_else(|| TypeError::NotFound {
                    src: env.src.clone(),
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

            env.nodes.insert(*id, ty_id);
            env.nodes.insert(ident.id, ty_id);

            Ok(ty_id)
        }

        Expr::MemberAccess(MemberAccess { lhs, ident, .. }) => {
            let lhs_ty_id = infer(lhs, env, scope)?;
            let field_ty_id = {
                let lhs_ty = env.types.get(&lhs_ty_id).unwrap();
                if let Type::Struct { fields, .. } = lhs_ty {
                    fields
                        .iter()
                        .find(|(field, _)| field.as_str() == ident.as_str()) // TODO: this is hacky D:
                        .map(|(_, ty_id)| *ty_id)
                        .ok_or_else(|| TypeError::NoSuchField {
                            src: env.src.clone(),
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
            let inner_ty_id = infer(expr, env, scope)?;
            Ok(env.types.intern(Type::Ptr(inner_ty_id)))
        }
    }?;

    env.nodes.insert(expr.id(), ty);

    Ok(ty)
}

pub fn infer_fn(func: &Fn, env: &mut TypeEnv, scope: &Scope<'_>) -> Result<(TypeId, DefId)> {
    match &func.kind {
        ast::FnKind::Local { params, body } => {
            let param_tys = params
                .iter()
                .map(|(_name, ty)| env.type_from_ast_ty(ty, scope))
                .collect::<Result<Vec<_>>>()?;
            let returns = env.type_from_ast_ty(&func.return_ty, scope)?;
            let fn_ty_id = env.types.intern(Type::func(param_tys, returns));
            let def_id = env.definitions.intern(fn_ty_id);

            let mut scope = scope.new_child();
            for (name, ty) in params.iter() {
                let ty_id = env.type_from_ast_ty(ty, &scope)?;
                let param_def_id = env.definitions.intern(ty_id);
                scope.define(name, param_def_id);
            }
            scope.define(&func.ident, def_id);

            let ty_id = infer_block(body, env, &mut scope)?;
            env.nodes.insert(body.id, ty_id);
            env.def_names.insert(def_id, func.ident.inner.clone());

            Ok((fn_ty_id, def_id))
        }
        ast::FnKind::Extern {
            params,
            is_variadic,
        } => {
            let param_tys = params
                .iter()
                .map(|(_name, ty)| env.type_from_ast_ty(ty, scope))
                .collect::<Result<Vec<_>>>()?;
            let returns = env.type_from_ast_ty(&func.return_ty, scope)?;
            let fn_ty_id = env
                .types
                .intern(Type::extern_func(param_tys, returns, *is_variadic));
            let def_id = env.definitions.intern(fn_ty_id);
            env.def_names.insert(def_id, func.ident.inner.clone());
            Ok((fn_ty_id, def_id))
        }
    }
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let { ident, ty, val, .. }) => {
            let ty_id = infer(val, env, scope)?;
            env.nodes.insert(ident.id, ty_id);
            env.nodes.insert(val.id(), ty_id);

            if let Some(declared_ty) = ty {
                let declared_ty_id = env.type_from_ast_ty(declared_ty, scope)?;
                if declared_ty_id != ty_id {
                    return Err(TypeError::InvalidType {
                        src: env.src.clone(),
                        span: val.span(),
                        expected: env.types.get(&declared_ty_id).unwrap().clone(),
                        actual: env.types.get(&ty_id).unwrap().clone(),
                    });
                }
            }

            let def_id = env.definitions.intern(ty_id);
            env.node_defs.insert(ident.id, def_id);
            env.def_names.insert(def_id, ident.inner.clone());
            scope.define(ident, def_id);
        }

        Stmnt::Ret(Ret { val, .. }) => {
            infer(val, env, scope)?;
        }

        Stmnt::Expr(expr) => {
            infer(expr, env, scope)?;
        }
    }

    Ok(())
}

pub fn check_func(func: &Fn, env: &mut TypeEnv, scope: &Scope<'_>) -> Result<()> {
    let mut scope = scope.new_child();

    for param in func.params() {
        let ty_id = env.type_from_ast_ty(param.ty, &scope)?;
        let def_id = env.definitions.intern(ty_id);
        if let Some(node_id) = param.node_id {
            env.nodes.insert(node_id, ty_id);
            env.node_defs.insert(node_id, def_id);
        }
        scope.define(param.key, def_id);
    }

    let def_id = *scope.get_definition(&func.ident).unwrap();
    let ty_id = *env.definitions.get(&def_id).unwrap();
    env.node_defs.insert(func.ident.id, def_id);
    env.nodes.insert(func.ident.id, ty_id);
    scope.define(&func.ident, def_id);

    match &func.kind {
        ast::FnKind::Local { body, .. } => check_stmnts(&body.nodes, env, &mut scope),
        ast::FnKind::Extern { .. } => Ok(()),
    }
}

pub fn check_imp(imp: &Impl, env: &mut TypeEnv, scope: &Scope<'_>) -> Result<()> {
    for item in imp.items.iter() {
        check_func(item, env, scope)?;
    }

    Ok(())
}

pub fn check_item(item: &Item, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match item {
        Item::Fn(func) => check_func(func, env, scope),
        Item::Impl(imp) => check_imp(imp, env, scope),
        _ => Ok(()),
        // Item::Use(Use { .. }) => {}
        // Item::StructDef(StructDef { .. }) => {}
    }
}

pub fn check_stmnts(stmnts: &[Stmnt], env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    for stmnt in stmnts {
        check_stmnt(stmnt, env, scope)?;
    }
    Ok(())
}

pub fn check_module(module: &Module, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    let mut inventory = collect(&module.items)?;

    for struct_def in inventory.take_structs() {
        let field_tys = struct_def
            .fields
            .iter()
            .map(|(name, ty)| Ok((name.to_owned(), env.type_from_ast_ty(ty, scope)?)))
            .collect::<Result<Vec<_>>>()?
            .into();
        let _impls = inventory.take_impls(&struct_def.ident); // TODO:

        let ty = Type::Struct {
            ident: struct_def.ident.to_owned().boxed(),
            fields: field_tys,
        };
        let ty_id = env.types.intern(ty);
        let def_id = env.definitions.intern(ty_id);
        scope.define(&struct_def.ident, def_id);
    }

    for func in inventory.take_fns() {
        let (_ty_id, def_id) = infer_fn(func, env, scope)?;
        scope.define(&func.ident, def_id);
    }

    for item in module.items.iter() {
        check_item(item, env, scope)?;
    }

    Ok(())
}
