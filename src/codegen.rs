use std::path::PathBuf;

use crate::ast::{Expr, Fn, InfixExpr, Node, Op, Stmnt};

pub trait Emitter {
    fn emit(&mut self, ast: Vec<Node>) -> String;
    fn emit_op(&mut self, buf: &mut String, op: &Op);
    fn emit_binop(&mut self, buf: &mut String, binop: &InfixExpr);
    fn emit_node(&mut self, buf: &mut String, node: &Node);
    fn emit_expr(&mut self, buf: &mut String, expr: &Expr);
    fn emit_stmnt(&mut self, buf: &mut String, stmnt: &Stmnt);
    fn emit_fn(&mut self, buf: &mut String, func: &Fn);
}

pub trait Compiler {
    type Opts;

    fn build_exe(&self, src: &str, program: &str, opts: Self::Opts) -> PathBuf;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum ReleaseType {
    Fast,
    #[default]
    Debug,
}

#[derive(Default)]
pub struct C {}

impl Emitter for C {
    fn emit(&mut self, ast: Vec<Node>) -> String {
        let mut buf = String::new();

        for node in ast {
            self.emit_node(&mut buf, &node);
        }

        buf
    }

    fn emit_op(&mut self, buf: &mut String, op: &Op) {
        let text = match op {
            Op::Add => "+",
            Op::Sub => "-",
        };
        buf.push_str(text);
    }

    fn emit_binop(&mut self, buf: &mut String, binop: &InfixExpr) {
        self.emit_expr(buf, binop.lhs.as_ref());
        self.emit_op(buf, &binop.op);
        self.emit_expr(buf, binop.rhs.as_ref());
    }

    fn emit_node(&mut self, buf: &mut String, node: &Node) {
        match node {
            Node::Expr(expr) => self.emit_expr(buf, expr),
            Node::Stmnt(stmnt) => self.emit_stmnt(buf, stmnt),
        }
    }

    fn emit_expr(&mut self, buf: &mut String, expr: &Expr) {
        match expr {
            Expr::IntLit(val) => buf.push_str(&val.to_string()),
            Expr::BinOp(binop) => self.emit_binop(buf, binop),
        };
    }

    fn emit_stmnt(&mut self, buf: &mut String, stmnt: &Stmnt) {
        match stmnt {
            Stmnt::Fn(func) => self.emit_fn(buf, func),
            Stmnt::Ret(expr) => {
                buf.push_str("return");
                buf.push(' ');
                self.emit_expr(buf, &expr);
                buf.push(';');
            }
        }
    }

    fn emit_fn(&mut self, buf: &mut String, func: &Fn) {
        buf.push_str(&func.return_ty);
        buf.push(' ');
        buf.push_str(&func.ident);
        buf.push_str("()");
        buf.push('{');
        for node in func.body.nodes.iter() {
            self.emit_node(buf, node);
        }
        buf.push('}');
    }
}

#[derive(Debug, Default)]
pub struct CCOpts {
    /// Delete all files created in the build process after emitting binary
    pub cleanup: bool,

    pub release: ReleaseType,
}

impl Compiler for C {
    type Opts = CCOpts;

    fn build_exe(&self, src: &str, program: &str, opts: Self::Opts) -> PathBuf {
        let out_path = PathBuf::from(format!("./{program}"));
        let tmp_src_path = PathBuf::from(format!("./{program}.c"));
        std::fs::write(&tmp_src_path, src).unwrap();

        let mut args = vec![
            tmp_src_path.to_str().unwrap(),
            "-o",
            out_path.to_str().expect("valid out path"),
        ];

        if opts.release == ReleaseType::Fast {
            args.extend_from_slice(&[
                "-O3",           // release optim
                "-march=native", // enable cpu specific instructions
                "-flto",         // link time opt
            ]);
        }

        let handle = std::process::Command::new("cc")
            .args(&args)
            .spawn()
            .expect("to start cc");

        let output = handle.wait_with_output().expect("cc failed to build");
        // dbg!(output.stderr, output.stdout);

        if opts.cleanup {
            std::fs::remove_file(tmp_src_path);
        }

        out_path
    }
}
