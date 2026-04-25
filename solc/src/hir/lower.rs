use std::borrow::Cow;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast;
use crate::hir::{self, HirId, Locality, Mutability};
use crate::lexer::source::{SourceInfo, Span};
use crate::traits::{Boxed, CollectVec, TransposeVec};
use crate::type_checker::collect::{CollectError, Inventory, collect};
use crate::type_checker::{FieldId, ItemId, MemberResolution, TypeEnv, TypeError, TypeId};

#[derive(Error, Diagnostic, Debug)]
pub enum LowerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Type(#[from] TypeError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Collect(#[from] CollectError),
    #[error("failed to resolve definition")]
    #[diagnostic(code(solc::hir::lower))]
    MissingDef {
        #[source_code]
        src: SourceInfo,

        #[label("here")]
        span: Span,
    },
}

pub type Result<T> = std::result::Result<T, LowerError>;

pub fn lower_func<'ast>(func: &'ast ast::Fn, env: &mut TypeEnv) -> Result<hir::Fn<'ast>> {
    let kind = match &func.kind {
        ast::FnKind::Local { params, body } => hir::FnKind::Local {
            params: params
                .iter()
                .map(|(ident, ty)| Ok((lower_ident(ident, env)?, env.type_of(&ty.id, &ty.span))))
                .collect::<Result<Vec<_>>>()?
                .into(),
            body: lower_block(body, env)?,
        },
        ast::FnKind::Extern {
            params,
            is_variadic,
        } => hir::FnKind::Extern {
            is_variadic: *is_variadic,
            params: params
                .iter()
                .map(|(name, ty)| (lower_name(name), env.type_of(&ty.id, &ty.span)))
                .collect_vec()
                .into(),
        },
    };

    Ok(hir::Fn {
        id: HirId::DUMMY,
        span: &func.span,
        ident: lower_ident(&func.ident, env)?,
        kind,
        return_ty: env.type_of(&func.return_ty.id, &func.span),
    })
}

pub fn lower_item<'ast>(
    item: &'ast ast::Item,
    inventory: &mut Inventory<'ast>,
    env: &mut TypeEnv,
) -> Result<Option<hir::Item<'ast>>> {
    Ok(match item {
        ast::Item::Use(inner) => Some(hir::Item::Use(hir::Use {
            id: HirId::DUMMY,
            span: &inner.span,
            locality: if inner.is_extern {
                Locality::Extern
            } else {
                Locality::Local
            },
            name: lower_name(&inner.name),
        })),
        ast::Item::Fn(func) => Some(hir::Item::Fn(lower_func(func, env)?)),
        ast::Item::StructDef(def) => {
            let fields = def
                .fields
                .iter()
                .map(|(_, ty)| env.type_of(&ty.id, &ty.span))
                .enumerate()
                .map(|(id, item)| (FieldId::from(id), item))
                .collect_vec();
            let items = inventory
                .take_impls(&def.ident)
                .into_iter()
                .flat_map(|imp| imp.items.as_ref())
                .map(|item| lower_assoc_item(item, env))
                .transpose_vec()?
                .into_iter()
                .enumerate()
                .map(|(id, item)| (ItemId::from(id), item))
                .collect_vec();

            Some(hir::Item::TyDef(hir::TyDef::Struct {
                id: HirId::DUMMY,
                span: &def.span,
                ident: lower_ident(&def.ident, env)?,
                fields: Box::from(fields),
                items: Box::from(items),
            }))
        }

        ast::Item::Impl(_) => None,
    })
}

pub fn lower_assoc_item<'ast>(
    item: &'ast ast::AssocItem,
    env: &mut TypeEnv,
) -> Result<hir::AssocItem<'ast>> {
    match item {
        ast::AssocItem::Fn(func) => lower_func(func, env).map(hir::AssocItem::Fn),
    }
}

pub fn lower_items<'ast>(
    items: &'ast [ast::Item],
    env: &mut TypeEnv,
) -> Result<Vec<hir::Item<'ast>>> {
    let mut inventory = collect(items)?;

    let items = items
        .iter()
        .filter_map(|item| lower_item(item, &mut inventory, env).transpose())
        .transpose_vec()?;

    Ok(items)
}

pub fn lower_module<'ast>(
    module: &'ast ast::Module,
    env: &mut TypeEnv,
) -> Result<hir::Module<'ast>> {
    Ok(hir::Module {
        items: lower_items(&module.items, env)?.into(),
    })
}

pub fn lower_block<'ast>(block: &'ast ast::Block, env: &mut TypeEnv) -> Result<hir::Block<'ast>> {
    Ok(hir::Block {
        id: HirId::DUMMY,
        ty: TypeId::NONE,
        span: &block.span,
        nodes: block
            .nodes
            .iter()
            .map(|stmnt| lower_stmnt(stmnt, env))
            .transpose_vec()?
            .into(),
    })
}

pub fn lower_name<'ast>(name: &'ast ast::Name) -> hir::Name<'ast> {
    hir::Name {
        id: HirId::DUMMY,
        span: &name.span,
        inner: &name.inner,
    }
}

pub fn lower_label<'ast>(label: &'ast ast::Label) -> hir::Label<'ast> {
    lower_name(label)
}

pub fn lower_ident<'ast>(ident: &'ast ast::Ident, env: &TypeEnv) -> Result<hir::Ident<'ast>> {
    let def_id = env
        .node_defs
        .get(&ident.id)
        .copied()
        .ok_or_else(|| LowerError::MissingDef {
            span: ident.span,
            src: env.src.clone(),
        })?;

    Ok(hir::Ident {
        id: HirId::DUMMY,
        def_id,
        ty: env.type_of(&ident.id, &ident.span),
        span: &ident.span,
        inner: &ident.inner,
        mutability: if env.mutable_definitions.contains(&def_id) {
            Mutability::Mutable
        } else {
            Mutability::Immutable
        },
    })
}

/// lower ident with a predetermined type
pub fn lower_typed_ident<'ast>(
    ident: &'ast ast::Ident,
    ty: TypeId,
    env: &TypeEnv,
) -> Result<hir::Ident<'ast>> {
    let def_id = env
        .node_defs
        .get(&ident.id)
        .copied()
        .ok_or_else(|| LowerError::MissingDef {
            span: ident.span,
            src: env.src.clone(),
        })?;

    Ok(hir::Ident {
        id: HirId::DUMMY,
        def_id,
        ty,
        span: &ident.span,
        inner: &ident.inner,
        mutability: Mutability::Immutable,
    })
}

/// lower identifier without inferring it's type.
pub fn lower_untyped_ident<'ast>(
    ident: &'ast ast::Ident,
    env: &TypeEnv,
) -> Result<hir::Ident<'ast>> {
    lower_typed_ident(ident, TypeId::NONE, env)
}

fn lower_member_access<'ast>(
    member_access: &'ast ast::MemberAccess,
    env: &mut TypeEnv,
) -> Result<hir::MemberAccess<'ast>> {
    let ty = env.type_of(&member_access.id, &member_access.span);
    let resolution = env
        .member_resolutions
        .get(&member_access.id)
        .copied()
        .expect("member access to have an associated resolution");

    Ok(hir::MemberAccess {
        id: HirId::DUMMY,
        ty,
        span: &member_access.span,
        lhs: lower_expr(&member_access.lhs, env)?.boxed(),
        resolution,
    })
}

pub fn lower_expr<'ast>(expr: &'ast ast::Expr, env: &mut TypeEnv) -> Result<hir::Expr<'ast>> {
    let ty = env.type_of(&expr.id(), &expr.span());
    let lowered = match expr {
        ast::Expr::Ident(ident) => hir::Expr::Ident(lower_ident(ident, env)?),
        ast::Expr::Literal(literal) => hir::Expr::Literal(hir::Literal {
            id: HirId::DUMMY,
            ty,
            span: &literal.span,
            kind: &literal.kind,
        }),
        ast::Expr::Block(block) => hir::Expr::Block(lower_block(block, env)?),
        ast::Expr::BinOp(bin_op) => hir::Expr::BinOp(hir::BinOp {
            id: HirId::DUMMY,
            ty,
            span: &bin_op.span,
            lhs: lower_expr(&bin_op.lhs, env)?.boxed(),
            op: &bin_op.op,
            rhs: lower_expr(&bin_op.rhs, env)?.boxed(),
        }),
        ast::Expr::Unary(unary) => hir::Expr::Unary(hir::Unary {
            id: HirId::DUMMY,
            ty,
            span: &unary.span,
            op: Cow::Borrowed(&unary.op),
            rhs: lower_expr(&unary.rhs, env)?.boxed(),
        }),
        ast::Expr::Call(call_expr) => {
            let def_id = env
                .node_defs
                .get(&call_expr.func.id())
                .copied()
                .expect("call target should have a resolved DefId");
            hir::Expr::Call(hir::Call {
                id: HirId::DUMMY,
                def_id,
                ty,
                span: &call_expr.span,
                func: lower_expr(&call_expr.func, env)?.boxed(),
                params: call_expr
                    .params
                    .iter()
                    .map(|param| lower_expr(param, env))
                    .transpose_vec()?
                    .into(),
            })
        }
        ast::Expr::Index(index_expr) => hir::Expr::Index(hir::Index {
            id: HirId::DUMMY,
            ty,
            span: &index_expr.span,
            expr: lower_expr(&index_expr.expr, env)?.boxed(),
            idx: lower_expr(&index_expr.idx, env)?.boxed(),
        }),
        ast::Expr::IfElse(if_else) => hir::Expr::IfElse(hir::IfElse {
            id: HirId::DUMMY,
            ty,
            span: &if_else.span,
            condition: lower_expr(&if_else.condition, env)?.boxed(),
            consequence: lower_block(&if_else.consequence, env)?,
            alternative: if_else
                .alternative
                .as_ref()
                .map(|block| lower_block(block, env))
                .transpose()?,
        }),
        ast::Expr::List(list) => hir::Expr::List(hir::List {
            id: HirId::DUMMY,
            ty,
            span: &list.span,
            items: list
                .items
                .iter()
                .map(|expr| lower_expr(expr, env))
                .transpose_vec()?
                .into(),
            size: list.items.len() as u64,
        }),
        ast::Expr::Constructor(constructor) => hir::Expr::Constructor(hir::Constructor {
            id: HirId::DUMMY,
            ty,
            span: &constructor.span,
            ident: lower_ident(&constructor.ident, env).unwrap(),
            fields: constructor
                .fields
                .iter()
                .map(|(name, expr)| Ok((lower_name(name), lower_expr(expr, env)?)))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::MemberAccess(member_access) => {
            lower_member_access(member_access, env).map(hir::Expr::MemberAccess)?
        }
        ast::Expr::Ref(expr) => hir::Expr::Ref(lower_expr(expr, env)?.into()),
        ast::Expr::Assign(assign) => hir::Expr::Assign(hir::Assign {
            id: HirId::DUMMY,
            span: &assign.span,
            lhs: lower_expr(&assign.lhs, env)?.boxed(),
            rhs: lower_expr(&assign.rhs, env)?.boxed(),
        }),
        ast::Expr::Break(inner) => hir::Expr::Break(hir::Break {
            id: HirId::DUMMY,
            ty,
            span: &inner.span,
            label: inner.label.as_ref().map(lower_label),
            val: inner
                .val
                .as_ref()
                .map(|expr| lower_expr(expr, env))
                .transpose()?
                .map(Box::new),
        }),
        ast::Expr::Continue(inner) => hir::Expr::Continue(hir::Continue {
            id: HirId::DUMMY,
            span: &inner.span,
            label: inner.label.as_ref().map(lower_label),
        }),
        ast::Expr::While(inner) => {
            let mut body = lower_block(&inner.consequence, env)?;
            let desugared_condition = hir::Stmnt::Expr(hir::Expr::IfElse(hir::IfElse {
                id: HirId::DUMMY,
                ty: TypeId::UNIT,
                span: &inner.consequence.span,
                condition: hir::Expr::Unary(hir::Unary {
                    id: HirId::DUMMY,
                    ty: TypeId::BOOL,
                    span: &inner.consequence.span,
                    op: Cow::Owned(ast::Op::new(ast::UnaryOpKind::Not, inner.condition.span())),
                    rhs: lower_expr(&inner.condition, env)?.boxed(),
                })
                .into(),
                consequence: hir::Block {
                    id: HirId::DUMMY,
                    ty: TypeId::UNIT,
                    span: &inner.consequence.span,
                    nodes: Box::from([hir::Stmnt::Expr(hir::Expr::Break(hir::Break {
                        id: HirId::DUMMY,
                        ty: TypeId::UNIT,
                        span: &inner.span,
                        label: None,
                        val: None, // TODO: can be return val from loop body?
                    }))]),
                },
                alternative: None,
            }));
            let nodes = [desugared_condition]
                .into_iter()
                .chain(body.nodes.clone())
                .collect_vec();
            body.nodes = Box::from(nodes);

            hir::Expr::Loop(hir::Loop {
                id: HirId::DUMMY,
                ty,
                span: &inner.span,
                label: inner.label.as_ref().map(lower_label),
                body,
            })
        }
    };
    Ok(lowered)
}

pub fn lower_stmnt<'ast>(stmnt: &'ast ast::Stmnt, env: &mut TypeEnv) -> Result<hir::Stmnt<'ast>> {
    Ok(match stmnt {
        ast::Stmnt::Let(inner) => {
            let ty = *env.nodes.get(&inner.val.id());
            let def_id = env
                .node_defs
                .get(&inner.ident.id)
                .copied()
                .expect("call target should have a resolved DefId");
            hir::Stmnt::Let(hir::Let {
                id: HirId::DUMMY,
                def_id,
                span: &inner.span,
                mutable: inner.mutable,
                ident: lower_ident(&inner.ident, env)?,
                ty,
                val: lower_expr(&inner.val, env)?,
            })
        }
        ast::Stmnt::Ret(inner) => {
            let val = lower_expr(&inner.val, env)?;
            let ty = *env.nodes.get(&inner.val.id());
            hir::Stmnt::Ret(hir::Ret {
                id: HirId::DUMMY,
                ty,
                span: inner.span,
                val,
            })
        }
        ast::Stmnt::Expr(expr) => hir::Stmnt::Expr(lower_expr(expr, env)?),
    })
}
