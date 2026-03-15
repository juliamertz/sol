use miette::Diagnostic;
use thiserror::Error;

use crate::ast;
use crate::ext::Boxed;
use crate::hir::collect::{CollectError, Inventory, collect};
use crate::hir::{self, HirId};
use crate::type_checker::ty::Type;
use crate::type_checker::{Scope, TypeEnv, TypeError, TypeId, check_stmnt, infer, infer_fn};

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

    for func in inventory.take_fns() {
        let ty = infer_fn(func, env);
        let type_id = env.types.intern(ty);
        let def_id = env.definitions.intern(type_id);
        scope.define(&func.ident, def_id);
    }

    for struct_def in inventory.take_structs() {
        let field_tys: Box<[(ast::Ident, TypeId)]> = struct_def
            .fields
            .iter()
            .map(|(ident, ty)| (ident.to_owned(), env.type_from_ast_ty(ty)))
            .collect();
        let ty = Type::Struct {
            ident: struct_def.ident.to_owned().boxed(),
            fields: field_tys,
        };
        let type_id = env.types.intern(ty);
        let def_id = env.definitions.intern(type_id);
        scope.define(&struct_def.ident, def_id);
    }

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
        ty: TypeId::NONE, //TODO:
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
        ty: TypeId::NONE, //TODO:
        span: &ident.span,
        inner: &ident.inner,
    })
}

pub fn lower_expr<'ast>(
    expr: &'ast ast::Expr,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<hir::Expr<'ast>> {
    let ty = infer(expr, env, scope)?;
    let lowered = match expr {
        ast::Expr::Ident(ident) => hir::Expr::Ident(lower_ident(ident, env, scope)?),
        ast::Expr::Literal(literal) => hir::Expr::Literal(hir::Literal {
            id: HirId::DUMMY,
            ty,
            span: &literal.span,
            kind: &literal.kind,
        }),
        ast::Expr::Block(block) => hir::Expr::Block(lower_block(block, env, scope)?),
        ast::Expr::BinOp(bin_op) => hir::Expr::BinOp(hir::BinOp {
            id: HirId::DUMMY,
            ty,
            span: &bin_op.span,
            lhs: lower_expr(&bin_op.lhs, env, scope)?.boxed(),
            op: &bin_op.op,
            rhs: lower_expr(&bin_op.rhs, env, scope)?.boxed(),
        }),
        ast::Expr::Prefix(prefix_expr) => hir::Expr::Prefix(hir::Prefix {
            id: HirId::DUMMY,
            ty,
            span: &prefix_expr.span,
            op: &prefix_expr.op,
            rhs: lower_expr(&prefix_expr.rhs, env, scope)?.boxed(),
        }),
        ast::Expr::Call(call_expr) => hir::Expr::Call(hir::Call {
            id: HirId::DUMMY,
            ty,
            span: &call_expr.span,
            func: lower_expr(&call_expr.func, env, scope)?.boxed(),
            params: call_expr
                .params
                .iter()
                .map(|param| lower_expr(param, env, scope))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::Index(index_expr) => hir::Expr::Index(hir::Index {
            id: HirId::DUMMY,
            ty,
            span: &index_expr.span,
            expr: lower_expr(&index_expr.expr, env, scope)?.boxed(),
            idx: lower_expr(&index_expr.idx, env, scope)?.boxed(),
        }),
        ast::Expr::IfElse(if_else) => hir::Expr::IfElse(hir::IfElse {
            id: HirId::DUMMY,
            ty,
            span: &if_else.span,
            condition: lower_expr(&if_else.condition, env, scope)?.boxed(),
            consequence: lower_block(&if_else.consequence, env, scope)?,
            alternative: if_else
                .alternative
                .as_ref()
                .map(|block| lower_block(block, env, scope))
                .transpose()?,
        }),
        ast::Expr::List(list) => hir::Expr::List(hir::List {
            id: HirId::DUMMY,
            ty,
            span: &list.span,
            items: list
                .items
                .iter()
                .map(|expr| lower_expr(expr, env, scope))
                .collect::<Result<Vec<_>>>()?
                .into(),
        }),
        ast::Expr::Constructor(constructor) => hir::Expr::Constructor(hir::Constructor {
            id: HirId::DUMMY,
            ty,
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
        ast::Expr::MemberAccess(member_access) => hir::Expr::MemberAccess(hir::MemberAccess {
            id: HirId::DUMMY,
            ty,
            span: &member_access.span,
            lhs: lower_expr(&member_access.lhs, env, scope)?.boxed(),
            ident: lower_ident(&member_access.ident, env, scope)?,
        }),
        ast::Expr::Ref(expr) => hir::Expr::Ref(lower_expr(expr, env, scope)?.into()),
        ast::Expr::RawIdent(_) => todo!(),
    };
    Ok(lowered)
}

pub fn lower_stmnt<'ast>(
    stmnt: &'ast ast::Stmnt,
    inventory: &mut Inventory<'ast>,
    env: &mut TypeEnv,
    scope: &mut Scope<'_>,
) -> Result<Option<hir::Stmnt<'ast>>> {
    check_stmnt(stmnt, env, scope)?;

    Ok(match stmnt {
        ast::Stmnt::Let(inner) => {
            let ty = env
                .nodes
                .get(&inner.val.id())
                .copied()
                .unwrap_or(TypeId::NONE);
            Some(hir::Stmnt::Let(hir::Let {
                id: HirId::DUMMY,
                span: &inner.span,
                ident: lower_ident(&inner.ident, env, scope)?,
                ty,
                val: lower_expr(&inner.val, env, scope)?,
            }))
        }
        ast::Stmnt::Ret(inner) => {
            let val = lower_expr(&inner.val, env, scope)?;
            let ty = env
                .nodes
                .get(&inner.val.id())
                .copied()
                .unwrap_or(TypeId::NONE);
            Some(hir::Stmnt::Ret(hir::Ret {
                id: HirId::DUMMY,
                ty,
                span: &inner.span,
                val,
            }))
        }
        ast::Stmnt::Use(inner) => Some(hir::Stmnt::Use(hir::Use {
            id: HirId::DUMMY,
            span: &inner.span,
            ident: lower_ident(&inner.ident, env, scope)?,
        })),
        ast::Stmnt::Fn(func) => {
            let mut scope = scope.new_child();

            for (ident, ty) in func.params.iter() {
                let type_id = env.type_from_ast_ty(ty);
                let def_id = env.definitions.intern(type_id);
                scope.define(ident, def_id);
            }

            let fn_ty = infer_fn(func, env);
            let type_id = env.types.intern(fn_ty);
            let def_id = env.definitions.intern(type_id);
            scope.define(&func.ident, def_id);

            let param_ids: Vec<_> = func
                .params
                .iter()
                .map(|(ident, ty)| {
                    Ok((
                        lower_ident(ident, env, &mut scope)?,
                        env.type_from_ast_ty(ty),
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

            Some(hir::Stmnt::Fn(hir::Fn {
                id: HirId::DUMMY,
                span: &func.span,
                is_extern: func.is_extern,
                ident: lower_ident(&func.ident, env, &mut scope)?,
                params: param_ids.into(),
                return_ty: env.type_from_ast_ty(&func.return_ty),
                body: func
                    .body
                    .as_ref()
                    .map(|body| lower_block(body, env, &mut scope))
                    .transpose()?,
            }))
        }
        ast::Stmnt::StructDef(def) => Some(hir::Stmnt::StructDef(hir::StructDef {
            id: HirId::DUMMY,
            span: &def.span,
            ident: lower_ident(&def.ident, env, scope)?,
            fields: def
                .fields
                .iter()
                .map(|(ident, ty)| Ok((lower_ident(ident, env, scope)?, env.type_from_ast_ty(ty))))
                .collect::<Result<Vec<_>>>()?
                .into(),
            impls: inventory.take_impls(&def.ident).into(),
        })),

        // struct impls are handled in the collect phase ^
        ast::Stmnt::Impl(_) => None,
    })
}
