use miette::Diagnostic;
use thiserror::Error;

use crate::ast;
use crate::ext::Boxed;
use crate::hir::{self, HirId};
use crate::type_checker::collect::{CollectError, Inventory, collect};
use crate::type_checker::{TypeEnv, TypeError, TypeId};

#[derive(Error, Diagnostic, Debug)]
pub enum LowerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Type(#[from] TypeError),
    #[error(transparent)]
    // #[diagnostic(transparent)]
    Collect(#[from] CollectError),
}

pub type Result<T> = std::result::Result<T, LowerError>;

pub fn lower_item<'ast>(
    item: &'ast ast::Item,
    inventory: &mut Inventory<'ast>,
    env: &mut TypeEnv,
) -> Result<Option<hir::Item<'ast>>> {
    Ok(match item {
        ast::Item::Use(inner) => Some(hir::Item::Use(hir::Use {
            id: HirId::DUMMY,
            span: &inner.span,
            is_extern: inner.is_extern,
            ident: if inner.is_extern {
                lower_untyped_ident(&inner.ident)
            } else {
                lower_ident(&inner.ident, env)?
            },
        })),
        ast::Item::Fn(func) => {
            let param_ids: Vec<_> = func
                .params
                .iter()
                .map(|(ident, ty)| {
                    let type_id = env.type_of(&ty.id);
                    (lower_typed_ident(ident, type_id), type_id)
                })
                .collect::<Vec<_>>();

            Some(hir::Item::Fn(hir::Fn {
                id: HirId::DUMMY,
                span: &func.span,
                is_extern: func.is_extern,
                ident: lower_untyped_ident(&func.ident),
                params: param_ids.into(),
                return_ty: env.type_of(&func.return_ty.id),
                body: func
                    .body
                    .as_ref()
                    .map(|body| lower_block(body, env))
                    .transpose()?,
            }))
        }
        ast::Item::StructDef(def) => Some(hir::Item::StructDef(hir::StructDef {
            id: HirId::DUMMY,
            span: &def.span,
            name: lower_name(&def.name),
            fields: def
                .fields
                .iter()
                .map(|(ident, ty)| Ok((lower_untyped_ident(ident), env.type_of(&ty.id))))
                .collect::<Result<Vec<_>>>()?
                .into(),
            impls: inventory.take_impls(&def.name).into(),
        })),
        ast::Item::Impl(_) => None,
    })
}

pub fn lower_items<'ast>(
    items: &'ast [ast::Item],
    env: &mut TypeEnv,
) -> Result<Vec<hir::Item<'ast>>> {
    let mut inventory = collect(items)?;

    let items = items
        .iter()
        .filter_map(|item| lower_item(item, &mut inventory, env).transpose())
        .collect::<Result<Vec<_>>>()?;

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
    let stmnts = block
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, stmnt)| {
            let lowered = lower_stmnt(stmnt, env)?;
            if idx != block.nodes.len() - 1 {
                return Ok(lowered);
            }
            let hir::Stmnt::Expr(expr) = lowered else {
                return Ok(lowered);
            };

            Ok(hir::Stmnt::Ret(hir::Ret {
                id: HirId::DUMMY,
                ty: *expr.type_id(),
                span: *expr.span(),
                val: expr,
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(hir::Block {
        id: HirId::DUMMY,
        ty: TypeId::NONE,
        span: &block.span,
        nodes: stmnts.into(),
    })
}

pub fn lower_name<'ast>(name: &'ast ast::Name) -> hir::Name<'ast> {
    hir::Name {
        id: HirId::DUMMY,
        span: &name.span,
        inner: &name.inner,
    }
}

pub fn lower_ident<'ast>(ident: &'ast ast::Ident, env: &mut TypeEnv) -> Result<hir::Ident<'ast>> {
    Ok(hir::Ident {
        id: HirId::DUMMY,
        ty: env.type_of(&ident.id),
        span: &ident.span,
        inner: &ident.inner,
    })
}

/// lower ident with a predetermined type
pub fn lower_typed_ident<'ast>(ident: &'ast ast::Ident, ty: TypeId) -> hir::Ident<'ast> {
    hir::Ident {
        id: HirId::DUMMY,
        ty,
        span: &ident.span,
        inner: &ident.inner,
    }
}

/// lower identifier without inferring it's type.
pub fn lower_untyped_ident<'ast>(ident: &'ast ast::Ident) -> hir::Ident<'ast> {
    lower_typed_ident(ident, TypeId::NONE)
}

pub fn lower_expr<'ast>(expr: &'ast ast::Expr, env: &mut TypeEnv) -> Result<hir::Expr<'ast>> {
    let ty = env.type_of(&expr.id());
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
            op: &unary.op,
            rhs: lower_expr(&unary.rhs, env)?.boxed(),
        }),
        ast::Expr::Call(call_expr) => hir::Expr::Call(hir::Call {
            id: HirId::DUMMY,
            ty,
            span: &call_expr.span,
            func: lower_expr(&call_expr.func, env)?.boxed(),
            params: call_expr
                .params
                .iter()
                .map(|param| lower_expr(param, env))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
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
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::Constructor(constructor) => hir::Expr::Constructor(hir::Constructor {
            id: HirId::DUMMY,
            ty,
            span: &constructor.span,
            ident: lower_ident(&constructor.ident, env)?,
            fields: constructor
                .fields
                .iter()
                .map(|(ident, expr)| Ok((lower_untyped_ident(ident), lower_expr(expr, env)?)))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::MemberAccess(member_access) => hir::Expr::MemberAccess(hir::MemberAccess {
            id: HirId::DUMMY,
            ty,
            span: &member_access.span,
            lhs: lower_expr(&member_access.lhs, env)?.boxed(),
            ident: lower_typed_ident(&member_access.ident, ty),
        }),
        ast::Expr::Ref(expr) => hir::Expr::Ref(lower_expr(expr, env)?.into()),
    };
    Ok(lowered)
}

pub fn lower_stmnt<'ast>(stmnt: &'ast ast::Stmnt, env: &mut TypeEnv) -> Result<hir::Stmnt<'ast>> {
    Ok(match stmnt {
        ast::Stmnt::Let(inner) => {
            let ty = env
                .nodes
                .get(&inner.val.id())
                .copied()
                .unwrap_or(TypeId::NONE);
            hir::Stmnt::Let(hir::Let {
                id: HirId::DUMMY,
                span: &inner.span,
                ident: lower_ident(&inner.ident, env)?,
                ty,
                val: lower_expr(&inner.val, env)?,
            })
        }
        ast::Stmnt::Ret(inner) => {
            let val = lower_expr(&inner.val, env)?;
            let ty = env
                .nodes
                .get(&inner.val.id())
                .copied()
                .unwrap_or(TypeId::NONE);
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
