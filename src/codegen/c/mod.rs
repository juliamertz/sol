use super::{Compiler, Emitter, ReleaseType};

use crate::BuildOpts;
use crate::ast::{CallExpr, Expr, Fn, InfixExpr, Node, Op, Stmnt};

use std::fs;
use std::hash::Hasher;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};
use wyhash2::WyHash;

#[derive(Default)]
pub struct C {}

impl Emitter for C {
    type Input = Vec<Node>;

    fn emit(&mut self, ast: &Self::Input) -> String {
        let mut buf = String::new();

        for node in ast {
            self.emit_node(&mut buf, node);
        }

        buf
    }
}

impl C {
    // namespace prefix to be used for all identifiers
    fn prefix(&self, ident: &str) -> String {
        format!("__newlang_{ident}_generated")
    }

    fn emit_op(&mut self, buf: &mut String, op: &Op) {
        let text = match op {
            Op::Eq => "==",
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Lt => "<",
            Op::Gt => ">",
            Op::And => "&&",
            Op::Or => "||",
        };
        buf.push_str(text);
    }

    fn emit_infix_expr(&mut self, buf: &mut String, infix_expr: &InfixExpr) {
        self.emit_expr(buf, infix_expr.lhs.as_ref());
        self.emit_op(buf, &infix_expr.op);
        self.emit_expr(buf, infix_expr.rhs.as_ref());
    }

    fn emit_call_expr(&mut self, buf: &mut String, call_expr: &CallExpr) {
        self.emit_expr(buf, &call_expr.func);
        buf.push('(');

        let mut args = String::new();
        for arg in call_expr.args.iter() {
            self.emit_expr(&mut args, arg);
            args.push(',');
        }

        // TODO: this is hacky, maybe we're better of returning a string from each emit fn
        buf.push_str(args.strip_suffix(",").unwrap_or(&args));
        buf.push(')');
    }

    fn emit_node(&mut self, buf: &mut String, node: &Node) {
        match node {
            Node::Expr(expr) => {
                self.emit_expr(buf, expr);
                buf.push(';');
            }
            Node::Stmnt(stmnt) => self.emit_stmnt(buf, stmnt),
        }
    }

    fn emit_expr(&mut self, buf: &mut String, expr: &Expr) {
        match expr {
            Expr::Ident(ident) => buf.push_str(&self.prefix(ident)),
            Expr::IntLit(val) => buf.push_str(&val.to_string()),
            Expr::StringLit(val) => buf.push_str(format!("\"{val}\"").as_str()),
            Expr::Infix(infix_expr) => self.emit_infix_expr(buf, infix_expr),
            Expr::Call(call_expr) => self.emit_call_expr(buf, call_expr),
            Expr::If(r#if) => {
                buf.push_str("if(");
                self.emit_expr(buf, &r#if.condition);
                buf.push_str("){");
                for node in &r#if.consequence.nodes {
                    self.emit_node(buf, node);
                }
                buf.push('}');
            }
            Expr::List(_list) => unimplemented!(),
        };
    }

    fn emit_stmnt(&mut self, buf: &mut String, stmnt: &Stmnt) {
        match stmnt {
            Stmnt::Fn(func) => self.emit_fn(buf, func),
            Stmnt::Use(r#use) => {
                buf.push_str(format!("#include <{}.h>\n", r#use.ident).as_str());
            }
            Stmnt::Ret(ret) => {
                buf.push_str("return");
                buf.push(' ');
                self.emit_expr(buf, &ret.val);
                buf.push(';');
            }
            Stmnt::Let(r#let) => {
                buf.push_str(&r#let.ty);
                buf.push(' ');
                buf.push_str(&r#let.ident);
                buf.push('=');
                self.emit_expr(buf, r#let.val.as_ref().unwrap()); // TODO: optional emit
                buf.push(';');
            }
        }
    }

    fn emit_fn(&mut self, buf: &mut String, func: &Fn) {
        buf.push_str(&func.return_ty);
        buf.push(' ');
        buf.push_str(&func.ident);
        buf.push('(');
        buf.push_str(
            func.args
                .iter()
                .map(|arg| format!("{} {}", arg.ty, arg.ident))
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        buf.push(')');
        buf.push('{');
        for node in func.body.nodes.iter() {
            self.emit_node(buf, node);
        }
        if &func.ident == "main"
            && !matches!(func.body.nodes.last().unwrap(), Node::Stmnt(Stmnt::Ret(_)))
        {
            buf.push_str("return 0;");
        }
        buf.push('}');
    }
}

impl Compiler for C {
    fn build_exe(&self, src: &str, program: &str, opts: &BuildOpts) -> Result<PathBuf> {
        let mut hasher = WyHash::with_seed(0);
        hasher.write(program.as_bytes());
        let program_hash = hasher.finish();

        let out_path = opts.outdir.join(program);
        let hash_path = opts.outdir.join("hash");
        let tmp_src_path = opts.outdir.join("source.c");

        if let Ok(hash) = fs::read_to_string(&hash_path) {
            if hash == format!("{program_hash:x}") {
                // TODO: verify hash of output binary, don't just assume it's right
                return Ok(out_path);
            }
        };

        fs::write(&tmp_src_path, src).unwrap();
        fs::write(&hash_path, format!("{program_hash:?}")).unwrap();

        let mut args = vec![
            tmp_src_path.to_str().unwrap(),
            "-Wall",
            "-Wextra",
            "-o",
            out_path.to_str().expect("valid out path"),
        ];

        if opts.release == ReleaseType::Fast {
            args.extend_from_slice(&[
                "-O3",   // turn on all optimizations
                "-flto", // link time optimization
            ]);
        }

        fs::create_dir_all(&opts.outdir).into_diagnostic()?;

        let handle = std::process::Command::new("cc")
            .args(&args)
            .spawn()
            .expect("to start cc");

        let _output = handle.wait_with_output().expect("cc failed to build");

        if opts.cleanup {
            fs::remove_file(tmp_src_path).unwrap();
        }

        Ok(out_path)
    }
}
