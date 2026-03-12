use miette::Diagnostic;
use thiserror::Error;

use crate::ast;
use crate::hir::collect::{CollectError, Inventory, collect};
use crate::hir::{self, HirId};
use crate::type_checker::{Scope, Type, TypeEnv, TypeError, infer};

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

    Ok(nodes.into())
}

pub fn lower_block<'ast>(
    block: &'ast ast::Block,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<hir::Block<'ast>> {
    Ok(hir::Block {
        id: HirId::DUMMY,
        nodes: lower_nodes(&block.nodes, env, scope)?.into(),
    })
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

pub fn lower_expr<'ast>(
    expr: &'ast ast::Expr,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<hir::Expr<'ast>> {
    let kind = match expr {
        ast::Expr::Ident(ident) => hir::ExprKind::Ident(ident),
        ast::Expr::Literal(literal) => hir::ExprKind::Literal(literal),
        ast::Expr::Block(block) => hir::ExprKind::Block(block),
        ast::Expr::BinOp(bin_op) => hir::ExprKind::BinOp(bin_op),
        ast::Expr::Prefix(prefix_expr) => hir::ExprKind::Prefix(prefix_expr),
        ast::Expr::Call(call_expr) => hir::ExprKind::Call(call_expr),
        ast::Expr::Index(index_expr) => hir::ExprKind::Index(index_expr),
        ast::Expr::IfElse(if_else) => hir::ExprKind::IfElse(if_else),
        ast::Expr::List(list) => hir::ExprKind::List(list),
        ast::Expr::Constructor(constructor) => hir::ExprKind::Constructor(constructor),
        ast::Expr::MemberAccess(member_access) => hir::ExprKind::MemberAccess(member_access),
        ast::Expr::Ref(expr) => hir::ExprKind::Ref(lower_expr(expr, env, scope)?.into()),
        ast::Expr::RawIdent(_) => todo!(),
    };
    let ty = infer(expr, env, scope)?;
    Ok(hir::Expr {
        hir_id: HirId::DUMMY,
        kind,
        ty,
        span: expr.span(),
    })
}

pub fn lower_stmnt<'ast>(
    stmnt: &'ast ast::Stmnt,
    inventory: &mut Inventory<'ast>,
    _env: &mut TypeEnv,
    _scope: &mut Scope<'_>,
) -> Result<Option<hir::Stmnt<'ast>>> {
    Ok(match stmnt {
        ast::Stmnt::Let(inner) => Some(hir::Stmnt::Let(inner)),
        ast::Stmnt::Ret(inner) => Some(hir::Stmnt::Ret(inner)),
        ast::Stmnt::Use(inner) => Some(hir::Stmnt::Use(inner)),
        ast::Stmnt::Fn(inner) => Some(hir::Stmnt::Fn(inner)),
        ast::Stmnt::StructDef(def) => Some(hir::Stmnt::StructDef {
            def,
            impls: inventory.take_impls(&def.ident).into(),
        }),
        _ => None,
    })
}
