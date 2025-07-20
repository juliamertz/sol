use crate::ast::Op;
use crate::codegen::Emitter;
use crate::hir::{Expr, Module, Node, Stmnt, Type, TypeEnv};

#[derive(Default)]
pub struct C;

impl Emitter for C {
    type Input = Module;

    fn emit(&mut self, module: &Self::Input) -> String {
        let mut buf = String::new();

        for node in module.nodes.iter() {
            match node {
                Node::Expr(expr) => emit_expr(module, &mut buf, expr),
                Node::Stmnt(stmnt) => emit_stmnt(module, &mut buf, stmnt),
            }
        }

        buf
    }
}

fn emit_node(module: &Module, buf: &mut String, node: &Node) {
    match node {
        Node::Expr(expr) => emit_expr(module, buf, expr),
        Node::Stmnt(stmnt) => emit_stmnt(module, buf, stmnt),
    }
    buf.push(';');
}

fn emit_op(buf: &mut String, op: &Op) {
    buf.push_str(match op {
        Op::Eq => "==",
        Op::Add => "+",
        Op::Sub => "-",
        Op::Mul => "*",
        Op::Div => "/",
        Op::Lt => "<",
        Op::Gt => ">",
        Op::And => "&&",
        Op::Or => "||",
        Op::Chain => ".",
    });
}

fn emit_type(buf: &mut String, ty: &Type) {
    buf.push_str(match ty {
        Type::Int => "int",
        Type::Str => "char *",
        Type::Bool => "bool",
        Type::List(_) => "List",
        Type::Struct { ident, .. } => ident,
        Type::Var(name) => name,
        Type::Ptr(_ty) => todo!(),
        Type::Fn { .. } | Type::Any => todo!(),
        Type::Unit => todo!(),
    })
}

fn emit_expr_list(module: &Module, buf: &mut String, exprs: &[Expr]) {
    for (idx, expr) in exprs.iter().enumerate() {
        emit_expr(module, buf, expr);
        if idx != exprs.len() - 1 {
            buf.push(',');
        }
    }
}

// fn emit_block(module: &Module, buf: &mut String, nodes: &[Node]) {
//     for node in nodes.iter() {
//         match node {
//             Node::Expr(expr) => {
//             },
//             _ => emit_node(module, buf, node),
//         }
//     }
// }

fn emit_expr(module: &Module, buf: &mut String, expr: &Expr) {
    match expr {
        Expr::IntLit(int) => buf.push_str(&int.to_string()),
        Expr::StrLit(str) => {
            buf.push('"');
            buf.push_str(str);
            buf.push('"');
        }
        Expr::Var { id, .. } => {
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
        }
        Expr::BinOp { lhs, op, rhs, .. } => {
            emit_expr(module, buf, lhs);
            emit_op(buf, op);
            emit_expr(module, buf, rhs);
        }
        Expr::Unary { op, rhs, .. } => {
            emit_op(buf, op);
            emit_expr(module, buf, rhs);
        }
        Expr::Block { nodes, .. } => {
            for node in nodes.iter() {
                emit_node(module, buf, node);
            }
        }
        Expr::Call { id, params, .. } => {
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
            buf.push('(');
            emit_expr_list(module, buf, params);
            buf.push(')');
        }
        Expr::Index { id, idx, .. } => {
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
            buf.push('[');
            buf.push_str(&idx.to_string());
            buf.push(']');
        }
        Expr::IfElse {
            condition,
            consequence,
            alternative,
            ty,
        } => {
            buf.push_str("({");

            if ty != &Type::Unit {
                emit_type(buf, ty);
                buf.push(' ');
                buf.push_str("tmpvar");
                buf.push(';');
            }

            buf.push_str("if");
            buf.push('(');
            emit_expr(module, buf, condition);
            buf.push(')');
            buf.push('{');
            for node in consequence.iter() {
                // TODO: assign last node expr to tmpvar
                emit_node(module, buf, node);
            }
            buf.push('}');
            if let Some(alternative) = alternative {
                buf.push_str("else");
                buf.push('{');
                for node in alternative.iter() {
                    // TODO: assign last node expr to tmpvar
                    emit_node(module, buf, node);
                }
                buf.push('}');
            }

            if ty != &Type::Unit {
                buf.push_str("tmpvar");
                buf.push(';');
            }

            buf.push_str("})");
        }
        Expr::List(exprs) => {
            buf.push('[');
            emit_expr_list(module, buf, exprs);
            buf.push(']');
        }
        Expr::Constructor { id, fields } => {
            buf.push_str("new");
            buf.push(' ');
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
            buf.push('(');
            // TODO: constructor params
            buf.push(')');
        }
        Expr::Ref(_) | Expr::Deref(_) => panic!("references are not supported for JS backend"),
    }
}

fn emit_stmnt(module: &Module, buf: &mut String, stmnt: &Stmnt) {
    match stmnt {
        Stmnt::Let { id, val } => {
            buf.push_str("let");
            buf.push(' ');
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
            buf.push('=');
            emit_expr(module, buf, val);
            buf.push(';');
        }
        Stmnt::Fn {
            id,
            r#extern,
            params,
            body,
            returns,
        } => {
            if *r#extern {
                return;
            }
            emit_type(buf, returns);
            buf.push(' ');
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
            buf.push('(');
            for (idx, param_id) in params.iter().enumerate() {
                let sym = module
                    .symbols
                    .get(*param_id as usize)
                    .expect("valid symbol");
                emit_type(buf, &sym.ty);
                buf.push(' ');
                buf.push_str(&sym.name);
                if idx != params.len() - 1 {
                    buf.push(',');
                }
            }
            buf.push(')');
            buf.push('{');
            for node in body.iter() {
                emit_node(module, buf, node);
            }
            buf.push('}');
        }
        Stmnt::Ret { implicit, val, ty } => {
            buf.push_str("return");
            buf.push(' ');
            if let Some(expr) = val {
                emit_expr(module, buf, expr);
            }
            buf.push(';');
        }
        Stmnt::Struct { id, impls } => {
            buf.push_str("class");
            buf.push(' ');
            let sym = module.symbols.get(*id as usize).expect("valid symbol");
            buf.push_str(&sym.name);
            buf.push('{');
            // TODO: class impl
            buf.push('}');
        }
        Stmnt::Use { path } => buf.push_str(&format!("#include <{}.h>\n", path.join(""))),
    }
}
