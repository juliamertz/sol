use crate::ast;
use crate::hir;
use crate::hir::HirId;
use crate::type_checker::Type;

pub fn lower(module: &ast::Module) -> hir::Module {
    let nodes = module.nodes.iter().map(lower_node).collect();
    hir::Module { nodes }
}

pub fn lower_node(node: &ast::Node) -> hir::Node {
    match node {
        ast::Node::Expr(expr) => hir::Node::Expr(lower_expr(expr)),
        ast::Node::Stmnt(stmnt) => hir::Node::Stmnt(lower_stmnt(stmnt)),
    }
}

pub fn lower_expr<'ast>(expr: &'ast ast::Expr) -> hir::Expr<'ast> {
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
        ast::Expr::Ref(expr) => hir::ExprKind::Ref(Box::new(lower_expr(expr))),
        ast::Expr::RawIdent(_) => todo!(),
    };
    hir::Expr {
        hir_id: HirId::DUMMY,
        kind,
        ty: Type::None,
        span: (0, 0).into(),
    }
}

pub fn lower_stmnt(stmnt: &ast::Stmnt) -> hir::Stmnt {
    match stmnt {
        ast::Stmnt::Let(_) => todo!(),
        ast::Stmnt::Ret(ret) => todo!(),
        ast::Stmnt::Use(_) => todo!(),
        ast::Stmnt::Fn(_) => todo!(),
        ast::Stmnt::StructDef(struct_def) => todo!(),
        ast::Stmnt::Impl(_) => todo!(),
    }
}
