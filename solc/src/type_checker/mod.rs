use std::collections::HashMap;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::{
    self, AssocItem, BinOp, BinOpKind, Block, Call, Constructor, Expr, Fn, Ident, IfElse, Index,
    Item, Let, List, Literal, LiteralKind, MemberAccess, Module, Name, NodeId, Ret, Stmnt,
    StructDef, Unary, UnaryOpKind, Use,
};
use crate::id;
use crate::interner::Interner;
use crate::lexer::source::{SourceInfo, Span};
use crate::traits::{AsStr, Boxed, TransposeVec};
use crate::type_checker::collect::{CollectError, collect};
use crate::type_checker::interner::TypeInterner;
use crate::type_checker::mangle::Mangle;
use crate::type_checker::ty::*;

pub mod collect;
pub mod interner;
pub mod mangle;
pub mod ty;

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(solc::type_checker))]
pub enum TypeError {
    #[diagnostic(forward(0))]
    #[error(transparent)]
    Collect(#[from] CollectError),

    #[error("`{ident}` not found in scope")]
    NotFound {
        #[source_code]
        src: SourceInfo,

        ident: Ident,

        #[label("this variable here")]
        span: Span,
    },

    #[error("no field `{name}` on type `{ty}`")]
    NoSuchField {
        #[source_code]
        src: SourceInfo,

        name: Name,

        ty: Ty,

        #[label("here")]
        span: Span,
    },

    #[error("invalid type, expected: {expected:?}, got: {actual:?}")]
    InvalidType {
        expected: Ty,
        actual: Ty,

        #[source_code]
        src: SourceInfo,

        #[label("here")]
        span: Span,
    },

    #[error("mismatched types in comparison")]
    ComparisonMismatch {
        #[source_code]
        src: SourceInfo,

        lhs_ty: Ty,
        #[label("has type `{lhs_ty}`")]
        lhs_span: Span,

        rhs_ty: Ty,
        #[label("has type `{rhs_ty}`")]
        rhs_span: Span,

        #[help]
        help: Option<String>,
    },

    #[error("mismatched element types in list")]
    HeterogeneousList {
        #[source_code]
        src: SourceInfo,

        first_ty: Ty,
        #[label("first element has type `{first_ty}`")]
        first_span: Span,

        other_ty: Ty,
        #[label("this element has type `{other_ty}`")]
        other_span: Span,

        #[help]
        help: Option<String>,
    },

    #[error("tried to access a member of non-aggregate type")]
    MemberAccessOnNonAggregate {
        #[source_code]
        src: SourceInfo,

        #[label("this node")]
        span: Span,
    },

    #[error("internal type checker error")]
    Internal,
}

type Result<T, E = TypeError> = core::result::Result<T, E>;

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

id!(FieldId);
id!(ItemId);

#[derive(Debug, Clone, Copy)]
pub enum MemberResolution {
    Field(FieldId),
    Item(ItemId),
}

#[derive(Debug)]
pub struct TypeEnv {
    pub(crate) src: SourceInfo,
    pub(crate) types: Interner<TypeId, Ty, TypeInterner>,
    pub(crate) associated_items: HashMap<(TypeId, String), (DefId, ItemId)>,
    pub(crate) member_resolutions: HashMap<NodeId, MemberResolution>,
    pub(crate) definitions: Interner<DefId, TypeId>,
    pub(crate) nodes: Interner<NodeId, TypeId>,
    pub(crate) mutable_definitions: Vec<DefId>,
    pub(crate) node_defs: HashMap<NodeId, DefId>,
    pub(crate) def_names: HashMap<DefId, Arc<str>>,
}

impl TypeEnv {
    pub fn new(src: SourceInfo) -> Self {
        Self {
            src,
            types: Interner::default(),
            associated_items: HashMap::default(),
            member_resolutions: HashMap::default(),
            definitions: Interner::default(),
            nodes: Interner::default(),
            mutable_definitions: Vec::default(),
            node_defs: HashMap::default(),
            def_names: HashMap::default(),
        }
    }

    pub fn type_of(&self, node_id: &NodeId, _span: &Span) -> TypeId {
        *self.nodes.get(node_id)
    }

    pub fn type_by_id(&self, type_id: &TypeId) -> Result<&Ty> {
        Ok(self.types.get(type_id))
    }

    pub fn type_from_ast_ty(&mut self, ast_ty: &ast::Ty, scope: &Scope<'_>) -> Result<TypeId> {
        let ty = match &ast_ty.kind {
            ast::TyKind::Int(kind) => Ty::Int(kind.into()),
            ast::TyKind::UInt(kind) => Ty::UInt(kind.into()),
            ast::TyKind::Bool => Ty::Bool,
            ast::TyKind::Str => Ty::Str,
            ast::TyKind::Var(ident) => {
                let def_id = scope.get_definition(ident).ok_or(TypeError::NotFound {
                    src: self.src.clone(),
                    ident: ident.to_owned(),
                    span: ident.span,
                })?;
                let ty_id = *self.definitions.get(def_id); // TODO: handle error
                self.nodes.insert(ast_ty.id, ty_id);
                return Ok(ty_id);
            }
            ast::TyKind::List { inner, size } => {
                let inner_id = self.type_from_ast_ty(inner, scope)?;
                Ty::List(inner_id, *size)
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
                Ty::Fn {
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
    let ty_id = *env.definitions.get(def_id);
    Ok(ty_id)
}

pub fn infer_block(block: &Block, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    check_stmnts(&block.nodes, env, scope)?;

    let ty_id = if let Some(last) = block.nodes.last() {
        match last {
            Stmnt::Expr(expr) => *env.nodes.get(&expr.id()),
            Stmnt::Ret(Ret { val, .. }) => *env.nodes.get(&val.id()),
            _ => env.types.intern(Ty::Unit),
        }
    } else {
        TypeId::NONE
    };

    env.nodes.insert(block.id, ty_id);

    Ok(ty_id)
}

pub fn infer_member_access(
    member_access: &MemberAccess,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<(TypeId, MemberResolution)> {
    let lhs_ty_id = infer(&member_access.lhs, env, scope)?;
    let lhs_ty = env.types.get(&lhs_ty_id);
    let struct_ty = lhs_ty
        .as_struct()
        .ok_or(TypeError::MemberAccessOnNonAggregate {
            src: env.src.clone(),
            span: member_access.span,
        })?;

    {
        let assoc_item = env
            .associated_items
            .get(&(lhs_ty_id, member_access.rhs.to_string()));

        if let Some((def_id, item_id)) = assoc_item.copied() {
            let ty_id = *env.definitions.get(&def_id);
            env.node_defs.insert(member_access.id, def_id);
            Ok((ty_id, MemberResolution::Item(item_id)))
        } else if let Some((field_id, ty_id)) = struct_ty.get_field(&member_access.rhs) {
            Ok((ty_id, MemberResolution::Field(field_id)))
        } else {
            Err(TypeError::NoSuchField {
                // TODO: or item...
                src: env.src.clone(),
                name: member_access.rhs.clone(),
                ty: lhs_ty.clone(),
                span: member_access.span,
            })
        }
    }
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
                BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Lt | BinOpKind::Gt => {
                    if lhs_ty != rhs_ty {
                        Err(TypeError::ComparisonMismatch {
                            src: env.src.clone(),
                            lhs_span: lhs.span(),
                            lhs_ty: env.types.get(&lhs_ty).clone(),
                            rhs_span: rhs.span(),
                            rhs_ty: env.types.get(&rhs_ty).clone(),
                            help: None,
                        })
                    } else {
                        Ok(TypeId::BOOL)
                    }
                }

                BinOpKind::And | BinOpKind::Or => {
                    if lhs_ty != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Ty::Bool,
                            actual: env.types.get(&lhs_ty).clone(),
                            src: env.src.clone(),
                            span: lhs.span(),
                        })
                    } else if rhs_ty != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Ty::Bool,
                            actual: env.types.get(&rhs_ty).clone(),
                            src: env.src.clone(),
                            span: rhs.span(),
                        })
                    } else {
                        Ok(TypeId::BOOL)
                    }
                }

                _ => {
                    let lhs_type = env.types.get(&lhs_ty);
                    match lhs_type {
                        Ty::Int(_) | Ty::UInt(_) => match op.kind {
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
            match (&op.kind, env.types.get(&ty)) {
                (UnaryOpKind::Negate, Ty::Int(_)) => Ok(ty),
                _ => todo!(),
            }
        }
        Expr::Call(Call { func, params, .. }) => {
            let func_ty_id = infer(func, env, scope)?;
            let returns = {
                let func_ty = env.types.get(&func_ty_id);
                let Ty::Fn { returns, .. } = func_ty else {
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
        Expr::Index(Index { id, expr, idx, .. }) => {
            let val_ty_id = infer(expr, env, scope)?;
            env.nodes.insert(expr.id(), val_ty_id);

            let idx_ty_id = infer(idx, env, scope)?;
            env.nodes.insert(idx.id(), idx_ty_id);

            let inner = {
                let ty = env.types.get(&val_ty_id);
                if let Ty::List(inner, _) = ty {
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
                    expected: Ty::Bool,
                    actual: env.types.get(&condition_ty).clone(),
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
                    lhs_ty: env.types.get(&consequence_ty).clone(),
                    rhs_span: alternative.span,
                    rhs_ty: env.types.get(&alternative_ty).clone(),
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
                        first_ty: env.types.get(&inner_type).clone(),
                        first_span: first_item.span(),
                        other_ty: env.types.get(&ty).clone(),
                        other_span: item.span(),
                        help: Some("pick a type and commit to it".into()),
                    });
                }
            }

            let ty = Ty::List(inner_type, Some(size));
            let ty_id = env.types.intern(ty);
            Ok(ty_id)
        }
        Expr::Constructor(Constructor {
            id, ident, fields, ..
        }) => {
            let def_id =
                scope
                    .get_definition(ident)
                    .copied()
                    .ok_or_else(|| TypeError::NotFound {
                        src: env.src.clone(),
                        ident: ident.to_owned(),
                        span: ident.span,
                    })?;
            let ty_id = *env.definitions.get(&def_id);

            for (_ident, expr) in fields.iter() {
                let _field_ty = infer(expr, env, scope)?;
                // TODO: validate fields
            }

            env.nodes.insert(*id, ty_id);
            env.nodes.insert(ident.id, ty_id);
            env.node_defs.insert(ident.id, def_id);

            Ok(ty_id)
        }
        Expr::MemberAccess(member_access) => {
            let (ty_id, resolution) = infer_member_access(member_access, env, scope)?;
            env.member_resolutions.insert(member_access.id, resolution);
            Ok(ty_id)
        }
        Expr::Ref(expr) => {
            let inner_ty_id = infer(expr, env, scope)?;
            Ok(env.types.intern(Ty::Ptr(inner_ty_id)))
        }
        Expr::Assign(assign) => {
            let _lhs_ty_id = infer(&assign.lhs, env, scope)?;
            let _rhs_ty_id = infer(&assign.rhs, env, scope)?;
            Ok(TypeId::UNIT)
        }
        Expr::Break(inner) => Ok(inner
            .val
            .as_ref()
            .map(|expr| infer(expr, env, scope))
            .transpose()?
            .unwrap_or(TypeId::UNIT)),
        Expr::Continue(_inner) => Ok(TypeId::UNIT),
        Expr::While(inner) => {
            let _cond_ty_id = infer(&inner.condition, env, scope)?;

            let (stmnts, returning) = inner.consequence.split_off_returning();
            for stmnt in stmnts {
                check_stmnt(stmnt, env, scope)?;
            }

            Ok(returning
                .map(|expr| infer(expr, env, scope))
                .transpose()?
                .unwrap_or(TypeId::UNIT))
        }
    }?;

    env.nodes.insert(expr.id(), ty);

    Ok(ty)
}

pub fn infer_func(func: &Fn, env: &mut TypeEnv, scope: &Scope<'_>) -> Result<(TypeId, DefId)> {
    match &func.kind {
        ast::FnKind::Local { params, body } => {
            let param_tys = params
                .iter()
                .map(|(_, ty)| env.type_from_ast_ty(ty, scope))
                .transpose_vec()?;
            let returns = env.type_from_ast_ty(&func.return_ty, scope)?;
            let fn_ty_id = env.types.intern(Ty::func(param_tys, returns));
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
                .intern(Ty::extern_func(param_tys, returns, *is_variadic));
            let def_id = env.definitions.intern(fn_ty_id);
            env.def_names.insert(def_id, func.ident.inner.clone());
            Ok((fn_ty_id, def_id))
        }
    }
}

pub fn infer_assoc_item(
    item: &AssocItem,
    env: &mut TypeEnv,
    scope: &Scope<'_>,
) -> Result<(TypeId, DefId)> {
    match item {
        AssocItem::Fn(func) => infer_func(func, env, scope),
    }
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let {
            ident,
            ty,
            val,
            mutable,
            ..
        }) => {
            let ty_id = infer(val, env, scope)?;
            env.nodes.insert(ident.id, ty_id);
            env.nodes.insert(val.id(), ty_id);

            if let Some(declared_ty) = ty {
                let declared_ty_id = env.type_from_ast_ty(declared_ty, scope)?;
                if declared_ty_id != ty_id {
                    return Err(TypeError::InvalidType {
                        src: env.src.clone(),
                        span: val.span(),
                        expected: env.types.get(&declared_ty_id).clone(),
                        actual: env.types.get(&ty_id).clone(),
                    });
                }
            }

            let def_id = env.definitions.intern(ty_id);
            if *mutable {
                env.mutable_definitions.push(def_id);
            }

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

pub fn check_func(func: &Fn, def_id: DefId, env: &mut TypeEnv, scope: &Scope<'_>) -> Result<()> {
    let mut fn_scope = scope.new_child();

    for param in func.params() {
        let ty_id = env.type_from_ast_ty(param.ty, &fn_scope)?;
        let def_id = env.definitions.intern(ty_id);
        if let Some(node_id) = param.node_id {
            env.nodes.insert(node_id, ty_id);
            env.node_defs.insert(node_id, def_id);
        }
        fn_scope.define(param.key, def_id);
    }

    let ty_id = *env.definitions.get(&def_id);
    env.node_defs.insert(func.ident.id, def_id);
    env.nodes.insert(func.ident.id, ty_id);
    fn_scope.define(&func.ident, def_id);

    match func.body() {
        Some(body) => check_stmnts(&body.nodes, env, &mut fn_scope),
        None => Ok(()),
    }
}

pub fn check_assoc_item(
    item: &AssocItem,
    def_id: DefId,
    env: &mut TypeEnv,
    scope: &Scope<'_>,
) -> Result<()> {
    match item {
        AssocItem::Fn(func) => check_func(func, def_id, env, scope),
    }
}

pub fn check_struct_def(def: &StructDef, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    let def_id = scope.get_definition(&def.ident).copied().unwrap();
    let ty_id = *env.definitions.get(&def_id);
    env.node_defs.insert(def.ident.id, def_id);
    env.nodes.insert(def.ident.id, ty_id);
    scope.define(&def.ident, def_id);

    Ok(())
}

pub fn check_use(_item: &Use, _env: &mut TypeEnv, _scope: &Scope<'_>) -> Result<()> {
    Ok(())
}

pub fn check_item(item: &Item, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match item {
        Item::Use(item) => check_use(item, env, scope),
        Item::Fn(func) => {
            let def_id = scope.get_definition(&func.ident).copied().unwrap();
            check_func(func, def_id, env, scope)
        }
        Item::Impl(_) => Ok(()),
        Item::StructDef(def) => check_struct_def(def, env, scope),
    }
}

pub fn check_stmnts(stmnts: &[Stmnt], env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    stmnts
        .iter()
        .map(|stmnt| check_stmnt(stmnt, env, scope))
        .transpose_vec()
        .map(|_| ())
}

pub fn check_module(module: &Module, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    let mut inventory = collect(&module.items)?;

    for struct_def in inventory.take_structs() {
        let ident = struct_def.ident.to_owned().boxed();
        let fields = struct_def
            .fields
            .iter()
            .map(|(name, ty)| Ok((name.to_owned(), env.type_from_ast_ty(ty, scope)?)))
            .collect::<Result<Vec<_>>>()?
            .into();

        let ty_id = env.types.intern(Ty::Struct(StructTy { ident, fields }));
        let def_id = env.definitions.intern(ty_id);
        scope.define(&struct_def.ident, def_id);

        {
            let scope = scope.new_child();
            let impls = inventory.take_impls(&struct_def.ident);
            for (idx, item) in impls.iter().flat_map(|imp| imp.items.as_ref()).enumerate() {
                let (_item_ty_id, def_id) = infer_assoc_item(item, env, &scope)?;

                let mangled = Mangle::AssocItem(struct_def.ident(), item.ident());
                let def_name = Arc::from(mangled.to_string());
                env.def_names.insert(def_id, def_name);

                let key = (ty_id, item.ident().to_string());
                let item_id = ItemId::from(idx);
                env.associated_items.insert(key, (def_id, item_id));

                check_assoc_item(item, def_id, env, &scope)?;
            }
        }
    }

    for func in inventory.take_fns() {
        let (_ty_id, def_id) = infer_func(func, env, scope)?;
        scope.define(&func.ident, def_id);
    }

    for item in module.items.iter() {
        check_item(item, env, scope)?;
    }

    Ok(())
}
