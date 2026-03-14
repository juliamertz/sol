use miette::Diagnostic;
use thiserror::Error;

use crate::ast;
use crate::ext::Boxed;
use crate::hir::collect::{CollectError, Inventory, collect};
use crate::hir::{self, HirId};
use crate::type_checker::{Scope, TypeEnv, TypeError, infer};

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

pub fn lower_node<'ast>(
    node: &'ast ast::Node,
    inventory: &mut Inventory<'ast>,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<Option<hir::Node<'ast>>> {
    Ok(match node {
        ast::Node::Expr(expr) => lower_expr(expr, env, scope).map(hir::Node::Expr)?.into(),
        ast::Node::Stmnt(stmnt) => lower_stmnt(stmnt, inventory, env, scope)?.map(hir::Node::Stmnt),
    })
}

pub fn lower_nodes<'ast>(
    nodes: &'ast [ast::Node],
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<Vec<hir::Node<'ast>>> {
    let mut inventory = collect(nodes)?;
    let nodes = nodes
        .iter()
        .filter_map(|node| lower_node(node, &mut inventory, env, scope).transpose())
        .collect::<Result<Vec<_>>>()?;

    Ok(nodes)
}

pub fn lower_module<'ast>(
    module: &'ast ast::Module,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<hir::Module<'ast>> {
    Ok(hir::Module {
        nodes: lower_nodes(&module.nodes, env, scope)?.into(),
    })
}

pub fn lower_block<'ast>(
    block: &'ast ast::Block,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<hir::Block<'ast>> {
    Ok(hir::Block {
        id: HirId::DUMMY,
        span: &block.span,
        nodes: lower_nodes(&block.nodes, env, scope)?.into(),
    })
}

pub fn lower_ident<'ast>(
    ident: &'ast ast::Ident,
    _env: &mut TypeEnv,
    _scope: &mut Scope<'_>,
) -> Result<hir::Ident<'ast>> {
    Ok(hir::Ident {
        id: HirId::DUMMY,
        span: &ident.span,
        inner: &ident.inner,
    })
}

pub fn lower_expr<'ast>(
    expr: &'ast ast::Expr,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<hir::Expr<'ast>> {
    let kind = match expr {
        ast::Expr::Ident(ident) => hir::ExprKind::Ident(lower_ident(ident, env, scope)?),
        ast::Expr::Literal(literal) => hir::ExprKind::Literal(hir::Literal {
            id: HirId::DUMMY,
            span: &literal.span,
            kind: &literal.kind,
        }),
        ast::Expr::Block(block) => hir::ExprKind::Block(lower_block(block, env, scope)?),
        ast::Expr::BinOp(bin_op) => hir::ExprKind::BinOp(hir::BinOp {
            id: HirId::DUMMY,
            span: &bin_op.span,
            lhs: lower_expr(&bin_op.lhs, env, scope)?.boxed(),
            op: &bin_op.op,
            rhs: lower_expr(&bin_op.rhs, env, scope)?.boxed(),
        }),
        ast::Expr::Prefix(prefix_expr) => hir::ExprKind::Prefix(hir::Prefix {
            id: HirId::DUMMY,
            span: &prefix_expr.span,
            op: &prefix_expr.op,
            rhs: lower_expr(&prefix_expr.rhs, env, scope)?.boxed(),
        }),
        ast::Expr::Call(call_expr) => hir::ExprKind::Call(hir::Call {
            id: HirId::DUMMY,
            span: &call_expr.span,
            func: lower_expr(&call_expr.func, env, scope)?.boxed(),
            params: call_expr
                .params
                .iter()
                .map(|param| lower_expr(param, env, scope))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::Index(index_expr) => hir::ExprKind::Index(hir::Index {
            id: HirId::DUMMY,
            span: &index_expr.span,
            expr: lower_expr(&index_expr.expr, env, scope)?.boxed(),
            idx: lower_expr(&index_expr.idx, env, scope)?.boxed(),
        }),
        ast::Expr::IfElse(if_else) => hir::ExprKind::IfElse(hir::IfElse {
            id: HirId::DUMMY,
            span: &if_else.span,
            condition: lower_expr(&if_else.condition, env, scope)?.boxed(),
            consequence: lower_block(&if_else.consequence, env, scope)?,
            alternative: if_else
                .alternative
                .as_ref()
                .map(|block| lower_block(block, env, scope))
                .transpose()?,
        }),
        ast::Expr::List(list) => hir::ExprKind::List(hir::List {
            id: HirId::DUMMY,
            span: &list.span,
            items: list
                .items
                .iter()
                .map(|expr| lower_expr(expr, env, scope))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::Constructor(constructor) => hir::ExprKind::Constructor(hir::Constructor {
            id: HirId::DUMMY,
            span: &constructor.span,
            ident: lower_ident(&constructor.ident, env, scope)?,
            fields: constructor
                .fields
                .iter()
                .map(|(ident, expr)| {
                    Ok((
                        lower_ident(ident, env, scope)?,
                        lower_expr(expr, env, scope)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::MemberAccess(member_access) => hir::ExprKind::MemberAccess(hir::MemberAccess {
            id: HirId::DUMMY,
            span: &member_access.span,
            lhs: lower_expr(&member_access.lhs, env, scope)?.boxed(),
            ident: lower_ident(&member_access.ident, env, scope)?,
        }),
        ast::Expr::Ref(expr) => hir::ExprKind::Ref(lower_expr(expr, env, scope)?.into()),
        ast::Expr::RawIdent(_) => todo!(),
    };
    let ty = infer(expr, env, scope)?;
    Ok(hir::Expr {
        kind,
        ty,
        span: expr.span(),
    })
}

pub fn lower_stmnt<'ast>(
    stmnt: &'ast ast::Stmnt,
    inventory: &mut Inventory<'ast>,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<Option<hir::Stmnt<'ast>>> {
    Ok(match stmnt {
        ast::Stmnt::Let(inner) => Some(hir::Stmnt::Let(hir::Let {
            id: HirId::DUMMY,
            span: &inner.span,
            ident: lower_ident(&inner.ident, env, scope)?,
            ty: inner.ty.as_ref(),
            val: lower_expr(&inner.val, env, scope)?,
        })),
        ast::Stmnt::Ret(inner) => Some(hir::Stmnt::Ret(hir::Ret {
            id: HirId::DUMMY,
            span: &inner.span,
            val: lower_expr(&inner.val, env, scope)?,
        })),
        ast::Stmnt::Use(inner) => Some(hir::Stmnt::Use(hir::Use {
            id: HirId::DUMMY,
            span: &inner.span,
            ident: lower_ident(&inner.ident, env, scope)?,
        })),
        ast::Stmnt::Fn(func) => Some(hir::Stmnt::Fn(hir::Fn {
            id: HirId::DUMMY,
            span: &func.span,
            is_extern: func.is_extern,
            ident: lower_ident(&func.ident, env, scope)?,
            params: func
                .params
                .iter()
                .map(|(ident, ty)| Ok((lower_ident(ident, env, scope)?, ty)))
                .collect::<Result<Vec<_>>>()?
                .into(),
            return_ty: &func.return_ty,
            body: func
                .body
                .as_ref()
                .map(|body| lower_block(&body, env, scope))
                .transpose()?,
        })),
        ast::Stmnt::StructDef(def) => Some(hir::Stmnt::StructDef(hir::StructDef {
            id: HirId::DUMMY,
            span: &def.span,
            ident: lower_ident(&def.ident, env, scope)?,
            fields: def
                .fields
                .iter()
                .map(|(ident, ty)| Ok((lower_ident(ident, env, scope)?, ty)))
                .collect::<Result<Vec<_>>>()?
                .into(),
            impls: inventory.take_impls(&def.ident).into(),
        })),

        // struct impls are handled in the collect phase ^
        ast::Stmnt::Impl(_) => None,
    })
}
