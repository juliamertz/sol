use std::collections::HashMap;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::{
    self, AssocItem, BinOp, BinOpKind, Block, Call, Constructor, Expr, Fn, Ident, IfElse, Impl,
    Index, Item, Let, List, Literal, LiteralKind, MemberAccess, Module, Name, NodeId, PathSegment,
    Ret, Stmnt, StructDef, Unary, UnaryOpKind, Use,
};
use crate::interner::{Id, Interner};
use crate::lexer::source::{SourceInfo, Span};
use crate::parser::resolve::{ModuleName, ModuleTree};
use crate::traits::{AsStr, Boxed, CollectVec, TransposeVec};
use crate::type_checker::collect::{CollectError, collect};
use crate::type_checker::fmt::TyDisplay;
use crate::type_checker::interner::TypeInterner;
use crate::type_checker::mangle::Mangle;
use crate::type_checker::ty::*;

pub mod collect;
pub mod fmt;
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

        ty: TyDisplay,

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

        lhs_ty: TyDisplay,
        #[label("has type `{lhs_ty}`")]
        lhs_span: Span,

        rhs_ty: TyDisplay,
        #[label("has type `{rhs_ty}`")]
        rhs_span: Span,

        #[help]
        help: Option<String>,
    },

    #[error("mismatched element types in list")]
    HeterogeneousList {
        #[source_code]
        src: SourceInfo,

        first_ty: TyDisplay,
        #[label("first element has type `{first_ty}`")]
        first_span: Span,

        other_ty: TyDisplay,
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

    #[error("cannot add `{first_ty}` to `{other_ty}`")]
    NonNumericOperand {
        #[source_code]
        src: SourceInfo,

        first_ty: TyDisplay,
        #[label("first element has type `{first_ty}`")]
        first_span: Span,

        other_ty: TyDisplay,
        #[label("this element has type `{other_ty}`")]
        other_span: Span,
    },

    #[error("internal type checker error")]
    Internal,
}

id!(DefId);
id!(TypeId);

pub type Result<T, E = TypeError> = core::result::Result<T, E>;

pub type Symbol = Arc<str>;
pub type SymbolMap = HashMap<Symbol, DefId>;

#[derive(Debug, Default)]
pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    definitions: SymbolMap,
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
    pub(crate) module_tree: ModuleTree,
    pub(crate) definitions: Interner<DefId, TypeId>,
    pub(crate) nodes: Interner<NodeId, TypeId>,
    pub(crate) mutable_definitions: Vec<DefId>,
    pub(crate) node_defs: HashMap<NodeId, DefId>,
    pub(crate) def_names: HashMap<DefId, Arc<str>>,
}

impl TypeEnv {
    pub fn new(src: SourceInfo, module_tree: ModuleTree) -> Self {
        Self {
            src,
            types: Interner::default(),
            associated_items: HashMap::default(),
            member_resolutions: HashMap::default(),
            module_tree,
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
            ast::TyKind::Float(kind) => Ty::Float(kind.into()),
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
            ast::TyKind::Ptr(inner) => Ty::Ptr(self.type_from_ast_ty(inner, scope)?),
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
        dbg!(&env.associated_items);
        dbg!(&lhs_ty_id, &member_access.rhs);
        let assoc_item = env
            .associated_items
            .get(&(lhs_ty_id, member_access.rhs.to_string()));
        dbg!(&assoc_item);

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
                ty: TyDisplay::new(lhs_ty, env),
                span: member_access.span,
            })
        }
    }
}

fn type_id_from_suffix(suffix: &ast::LiteralSuffix) -> TypeId {
    match suffix {
        ast::LiteralSuffix::Int(int_ty) => int_ty.into(),
        ast::LiteralSuffix::UInt(uint_ty) => uint_ty.into(),
        ast::LiteralSuffix::Float(float_ty) => float_ty.into(),
    }
}

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<TypeId> {
    let ty = match expr {
        Expr::Ident(ident) => infer_ident(ident, env, scope),

        Expr::Literal(Literal {
            id, kind, suffix, ..
        }) => match kind {
            LiteralKind::Str(_) => {
                let ty_id = TypeId::STR;
                env.nodes.insert(*id, ty_id);
                Ok(ty_id)
            }
            LiteralKind::Int(_) => {
                let ty_id: TypeId = suffix
                    .as_ref()
                    .map(type_id_from_suffix)
                    .unwrap_or(TypeId::I32); // TODO: infer the correct type if ommited
                env.nodes.insert(*id, ty_id);
                Ok(ty_id)
            }
            LiteralKind::Float(_) => {
                let ty_id: TypeId = suffix
                    .as_ref()
                    .map(type_id_from_suffix)
                    // TODO: i still need a way to infer these types. for now
                    // im defaulting to f64 because f32's are quite annoying to work with in qbe
                    .unwrap_or(TypeId::F64);

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
            let lhs_ty_id = infer(lhs.as_ref(), env, scope)?;
            let rhs_ty_id = infer(rhs.as_ref(), env, scope)?;

            let lhs_ty = env.types.get(&lhs_ty_id);
            let rhs_ty = env.types.get(&rhs_ty_id);

            match op.kind {
                BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Lt | BinOpKind::Gt => {
                    if lhs_ty_id != rhs_ty_id {
                        Err(TypeError::ComparisonMismatch {
                            src: env.src.clone(),
                            lhs_span: lhs.span(),
                            lhs_ty: TyDisplay::new(lhs_ty, env),
                            rhs_span: rhs.span(),
                            rhs_ty: TyDisplay::new(rhs_ty, env),
                            help: None,
                        })
                    } else {
                        Ok(TypeId::BOOL)
                    }
                }

                BinOpKind::And | BinOpKind::Or => {
                    if lhs_ty_id != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Ty::Bool,
                            actual: env.types.get(&lhs_ty_id).clone(),
                            src: env.src.clone(),
                            span: lhs.span(),
                        })
                    } else if rhs_ty_id != TypeId::BOOL {
                        Err(TypeError::InvalidType {
                            expected: Ty::Bool,
                            actual: env.types.get(&rhs_ty_id).clone(),
                            src: env.src.clone(),
                            span: rhs.span(),
                        })
                    } else {
                        Ok(TypeId::BOOL)
                    }
                }

                BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul | BinOpKind::Div => {
                    if lhs_ty.is_number() && rhs_ty.is_number() {
                        if lhs_ty_id == rhs_ty_id {
                            Ok(lhs_ty_id)
                        } else {
                            return Err(TypeError::ComparisonMismatch {
                                src: env.src.clone(),
                                lhs_ty: TyDisplay::new(lhs_ty, env),
                                lhs_span: lhs.span(),
                                rhs_ty: TyDisplay::new(rhs_ty, env),
                                rhs_span: rhs.span(),
                                help: None,
                            });
                        }
                    } else {
                        match (lhs_ty, rhs_ty) {
                            (Ty::Ptr(_), Ty::UInt(UIntTy::U64)) => Ok(lhs_ty_id),
                            _ => {
                                return Err(TypeError::NonNumericOperand {
                                    src: env.src.clone(),
                                    first_ty: TyDisplay::new(lhs_ty, env),
                                    first_span: lhs.span(),
                                    other_ty: TyDisplay::new(rhs_ty, env),
                                    other_span: rhs.span(),
                                });
                            }
                        }
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
            let consequence_ty_id = infer(&Expr::Block(consequence.to_owned()), env, block_scope)?;
            let alternative_ty = alternative
                .clone()
                .map(|alternative| infer(&Expr::Block(alternative), env, block_scope))
                .transpose()?;

            if let Some(alternative_ty_id) = alternative_ty
                && let Some(alternative) = alternative
                && alternative_ty_id != consequence_ty_id
            {
                let consequence_ty = env.types.get(&consequence_ty_id);
                let alternative_ty = env.types.get(&alternative_ty_id);
                return Err(TypeError::ComparisonMismatch {
                    src: env.src.clone(),
                    lhs_span: consequence.span,
                    lhs_ty: TyDisplay::new(consequence_ty, env),
                    rhs_span: alternative.span,
                    rhs_ty: TyDisplay::new(alternative_ty, env),
                    help: None,
                });
            }

            Ok(consequence_ty_id)
        }

        Expr::List(List { items, .. }) => {
            let size = items.len();
            let mut iter = items.iter();
            let first_item = iter.next();

            let inner_ty_id = first_item
                .map(|expr| infer(expr, env, scope))
                .transpose()?
                .unwrap_or(TypeId::NONE);

            while let Some(item) = iter.next()
                && let Some(first_item) = first_item
            {
                let ty_id = infer(item, env, scope)?;
                if ty_id != inner_ty_id {
                    let inner_ty = env.types.get(&inner_ty_id);
                    let ty = env.types.get(&ty_id);
                    return Err(TypeError::HeterogeneousList {
                        src: env.src.clone(),
                        first_ty: TyDisplay::new(inner_ty, env),
                        first_span: first_item.span(),
                        other_ty: TyDisplay::new(ty, env),
                        other_span: item.span(),
                        help: Some("pick a type and commit to it".into()),
                    });
                }
            }

            let ty = Ty::List(inner_ty_id, Some(size));
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

        Expr::Deref(expr) => infer(expr, env, scope),

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

pub fn infer_fn_from_signature(
    func: &Fn,
    env: &mut TypeEnv,
    scope: &Scope<'_>,
) -> Result<(TypeId, DefId)> {
    let (ty_id, def_id) = match &func.kind {
        ast::FnKind::Local { params, .. } => {
            let param_tys = params
                .iter()
                .map(|(_, ty)| env.type_from_ast_ty(ty, scope))
                .transpose_vec()?;
            let returns = env.type_from_ast_ty(&func.return_ty, scope)?;
            let ty_id = env.types.intern(Ty::func(param_tys, returns));
            let def_id = env.definitions.intern(ty_id);
            env.def_names.insert(def_id, func.ident.inner.clone());
            (ty_id, def_id)
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
            let ty_id = env
                .types
                .intern(Ty::extern_func(param_tys, returns, *is_variadic));
            let def_id = env.definitions.intern(ty_id);
            env.def_names.insert(def_id, func.ident.inner.clone());
            (ty_id, def_id)
        }
    };

    env.nodes.insert(func.ident.id, ty_id);
    env.node_defs.insert(func.ident.id, def_id);

    Ok((ty_id, def_id))
}

pub fn infer_fn_body(
    def_id: DefId,
    func: &Fn,
    env: &mut TypeEnv,
    scope: &Scope<'_>,
) -> Result<TypeId> {
    if let ast::FnKind::Local { params, body } = &func.kind {
        let mut scope = scope.new_child();
        for (ident, ty) in params.iter() {
            let param_ty_id = env.type_from_ast_ty(ty, &scope)?;
            let param_def_id = env.definitions.intern(param_ty_id);
            env.nodes.insert(ident.id, param_ty_id);
            env.node_defs.insert(ident.id, param_def_id);
            scope.define(ident, param_def_id);
        }
        scope.define(&func.ident, def_id);

        let ty_id = infer_block(body, env, &mut scope)?;
        env.nodes.insert(body.id, ty_id);
        Ok(ty_id)
    } else {
        Ok(TypeId::UNIT)
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
    // TODO: can be removed mayhaps
    // this stuff is now all handlded in `check_module`
    Ok(())
}

pub fn check_use(item: &Use, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    // for now we'll put everything from the module into our current scope

    let PathSegment::Named(name) = item
        .path
        .segments()
        .into_iter()
        .next()
        .expect("single named pathsegment");
    let module = env
        .module_tree
        .root()
        .children()
        .get(name.as_str())
        .unwrap()
        .clone(); // FIXME: really expensive clone here, please borrow checker later

    check_module(&module, env, scope)?;

    todo!();

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

fn declare_struct_names<'a>(
    struct_defs: &[&'a StructDef],
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<Vec<(DefId, TypeId, &'a StructDef)>> {
    let mut defined = vec![];

    for struct_def in struct_defs {
        let placeholder_ty = Ty::Struct(StructTy::placeholder(struct_def.ident.clone()));
        let ty_id = env.types.intern_no_insert(&placeholder_ty);
        let def_id = env.definitions.intern(ty_id);
        scope.define(&struct_def.ident, def_id);

        dbg!(ty_id);
        defined.push((def_id, ty_id, *struct_def));
    }

    Ok(defined)
}

fn resolve_structs(
    struct_defs: &[(DefId, TypeId, &StructDef)],
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<()> {
    for (_, ty_id, struct_def) in struct_defs {
        let fields = struct_def
            .fields
            .iter()
            .map(|(name, ty)| Ok((name.to_owned(), env.type_from_ast_ty(ty, scope)?)))
            .collect::<Result<Vec<_>>>()?
            .into();
        let ty = Ty::Struct(StructTy {
            ident: struct_def.ident.clone().boxed(),
            fields,
        });
        dbg!(ty_id);
        env.types.insert(*ty_id, ty);
    }

    Ok(())
}

fn declare_fn_signatures<'a>(
    fns: Vec<&'a Fn>,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<Vec<(DefId, &'a Fn)>> {
    Ok(fns
        .into_iter()
        .map(|func| {
            let (_ty_id, def_id) = infer_fn_from_signature(func, env, scope)?;
            // TODO: maybe should also just set the type here instead of in `check_func()`?
            scope.define(&func.ident, def_id);
            Ok((def_id, func))
        })
        .collect::<Result<Vec<_>>>()?)
}

fn declare_assoc_fn_signature(
    item: &AssocItem,
    item_id: ItemId,
    def: &StructDef,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<()> {
    if let AssocItem::Fn(func) = item {
        let (ty_id, def_id) = infer_fn_from_signature(func, env, &scope)?;
        let mangler = Mangle::AssocItem(def.ident(), item.ident());
        let def_name = Arc::from(mangler.to_string());
        env.def_names.insert(def_id, def_name);
        let key = (ty_id, item.ident().to_string());
        env.associated_items.insert(key, (def_id, item_id));
    }

    Ok(())
}

pub fn check_module(module: &Module, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    let mut inventory = collect(&module.items)?;

    // for item in module.items.iter() {
    //     if let Item::Use(stmnt) = item {
    //         // TODO: type check imported module
    //     };
    // }

    let structs = inventory.take_structs();
    let declared_structs = declare_struct_names(&structs, env, scope)?;
    resolve_structs(&declared_structs, env, scope)?;

    let fns = inventory.take_fns();
    let declared_fns = declare_fn_signatures(fns, env, scope)?;

    for (def_id, _, struct_def) in declared_structs {
        let impls = inventory.take_impls(&struct_def.ident);
        let items = impls
            .into_iter()
            .flat_map(|imp| imp.items.as_ref())
            .enumerate()
            .map(|(idx, item)| (ItemId::from(idx), item))
            .collect_vec();

        for (id, item) in items.iter() {
            declare_assoc_fn_signature(item, *id, struct_def, env, scope)?;
        }
        for (_, item) in items.iter() {
            let AssocItem::Fn(func) = item;
            infer_fn_body(def_id, func, env, scope)?;
        }
    }

    for (def_id, func) in declared_fns {
        infer_fn_body(def_id, func, env, scope)?;
    }

    Ok(())
}
