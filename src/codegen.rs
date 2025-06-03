use crate::ast::{BinOp, Expr, Op};

pub trait Emitter {
    fn emit(&mut self, ast: Vec<Expr>) -> String;
    fn emit_op(&mut self, buf: &mut String, op: &Op);
    fn emit_binop(&mut self, buf: &mut String, binop: &BinOp);
    fn emit_expr(&mut self, buf: &mut String, expr: &Expr);
}

#[derive(Default)]
pub struct C {}

impl Emitter for C {
    fn emit(&mut self, ast: Vec<Expr>) -> String {
        // Temporary function wrapper so we can run expressions without implementing functions
        let mut buf = String::new();

        for expr in ast {
            self.emit_expr(&mut buf, &expr);
        }

        let main = format!(
            r#"
            int main() {{
                {buf}

                return 0;
            }}
            "#
        );

        main
    }

    fn emit_op(&mut self, buf: &mut String, op: &Op) {
        let text = match op {
            Op::Add => "+",
            Op::Sub => "-",
        };
        buf.push_str(text);
    }

    fn emit_binop(&mut self, buf: &mut String, binop: &BinOp) {
        self.emit_expr(buf, binop.lhs.as_ref());
        self.emit_op(buf, &binop.op);
        self.emit_expr(buf, binop.rhs.as_ref());
    }

    fn emit_expr(&mut self, buf: &mut String, expr: &Expr) {
        match expr {
            Expr::IntLit(val) => buf.push_str(&val.to_string()),
            Expr::BinOp(binop) => self.emit_binop(buf, binop),
        };
    }
}
