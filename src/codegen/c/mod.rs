use crate::BuildOpts;
use crate::analyzer::{self, Analyzer, Checked, TypeEnv};
use crate::ast::{self, CallExpr, Expr, Fn, InfixExpr, Node, Op, Stmnt};
use crate::codegen::{Compiler, Emitter};

use std::fs;
use std::hash::Hasher;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};
use wyhash2::WyHash;

const CORE_INCLUDE_PATH: &str = "/Users/julia/projects/2025/newlang/src/codegen/c/include";
const CORE_INCLUDES: &[&str] = &["gc.h", "list.h"];

#[derive(Default)]
pub struct C {}

impl Emitter for C {
    type Input = Vec<Node>;

    fn emit(&mut self, ast: &Self::Input) -> String {
        let mut buf = String::new();
        let mut env = TypeEnv::new();

        Analyzer::collect_declarations(ast, &mut env).unwrap();

        for file in CORE_INCLUDES {
            buf.push_str(&format!("#include \"{CORE_INCLUDE_PATH}/{file}\"\n"));
        }

        for node in ast {
            self.emit_node(&mut buf, &mut env, node);
        }

        buf
    }
}

impl C {
    // namespace prefix to be used for all identifiers
    fn prefix(&self, ident: &str) -> String {
        // TODO: make sure where set up for isolation from other c code
        format!("__newlang_{ident}")
        // ident.to_string()
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

    fn emit_infix_expr(&mut self, buf: &mut String, env: &mut TypeEnv, infix_expr: &InfixExpr) {
        self.emit_expr(buf, env, infix_expr.lhs.as_ref());
        self.emit_op(buf, &infix_expr.op);
        self.emit_expr(buf, env, infix_expr.rhs.as_ref());
    }

    fn emit_call_expr(&mut self, buf: &mut String, env: &mut TypeEnv, call_expr: &CallExpr) {
        let name = match call_expr.func.as_ref() {
            Expr::Ident(ident) => ident,
            _ => todo!(),
        };
        let declaration = env.get(name);

        if let Some(Checked::Known(analyzer::Type::Fn {
            is_extern: true, ..
        })) = declaration
        {
            buf.push_str(name);
        } else {
            self.emit_expr(buf, env, &call_expr.func);
        }

        buf.push('(');

        let mut args = String::new();
        for arg in call_expr.args.iter() {
            self.emit_expr(&mut args, env, arg);
            args.push(',');
        }

        // TODO: this is hacky, maybe we're better of returning a string from each emit fn
        buf.push_str(args.strip_suffix(",").unwrap_or(&args));
        buf.push(')');
    }

    fn emit_node(&mut self, buf: &mut String, env: &mut TypeEnv, node: &Node) {
        match node {
            Node::Expr(expr) => {
                self.emit_expr(buf, env, expr);
                buf.push(';');
            }
            Node::Stmnt(stmnt) => self.emit_stmnt(buf, env, stmnt),
        }
    }

    fn emit_type(&mut self, env: &mut TypeEnv, ty: &ast::Type) -> String {
        match ty {
            ast::Type::Int => "int",
            ast::Type::Str => "char *",
            ast::Type::Bool => "bool",
            ast::Type::List(_) => "List",
            _ => unimplemented!(),
        }
        .into()
    }

    fn emit_expr(&mut self, buf: &mut String, env: &mut TypeEnv, expr: &Expr) {
        match expr {
            Expr::Ident(ident) => buf.push_str(&self.prefix(ident)),
            Expr::IntLit(val) => buf.push_str(&val.to_string()),
            Expr::StringLit(val) => buf.push_str(format!("\"{val}\"").as_str()),
            Expr::Infix(infix_expr) => self.emit_infix_expr(buf, env, infix_expr),
            Expr::Call(call_expr) => self.emit_call_expr(buf, env, call_expr),
            Expr::If(r#if) => {
                buf.push_str("if(");
                self.emit_expr(buf, env, &r#if.condition);
                buf.push_str("){");
                for node in &r#if.consequence.nodes {
                    self.emit_node(buf, env, node);
                }
                buf.push('}');
                if let Some(ref alternative) = r#if.alternative {
                    buf.push_str("else{");
                    for node in &alternative.nodes {
                        self.emit_node(buf, env, node);
                    }
                    buf.push('}');
                }
            }
            Expr::List(_list) => unimplemented!(),
        };
    }

    fn emit_stmnt(&mut self, buf: &mut String, env: &mut TypeEnv, stmnt: &Stmnt) {
        let checked = Analyzer::check_stmnt(stmnt, env).unwrap();

        match stmnt {
            Stmnt::Fn(func) => self.emit_fn(buf, env, func),
            Stmnt::Use(r#use) => {
                buf.push_str(format!("#include <{}.h>\n", r#use.ident).as_str());
            }
            Stmnt::Ret(ret) => {
                buf.push_str("return");
                buf.push(' ');
                self.emit_expr(buf, env, &ret.val);
                buf.push(';');
            }
            Stmnt::Let(binding) => {
                let ty = match checked {
                    Checked::Known(ref ty) => ty.into(),
                    Checked::Unknown => unreachable!(),
                };

                buf.push_str(self.emit_type(env, &ty).as_str());
                buf.push(' ');
                buf.push_str(&self.prefix(&binding.name));
                buf.push('=');

                match binding.val.as_ref().unwrap() {
                    Expr::List(_) => {
                        buf.push_str(
                            format!(
                                "list_alloc(sizeof({ty}), 64)",
                                ty = self.emit_type(env, &ty)
                            )
                            .as_str(),
                        );
                    }
                    _ => self.emit_expr(buf, env, binding.val.as_ref().unwrap()),
                };

                buf.push(';');
            }
        }
    }

    fn emit_fn(&mut self, buf: &mut String, env: &mut TypeEnv, func: &Fn) {
        if func.is_extern {
            return;
        }

        buf.push_str(&self.emit_type(env, &func.return_ty));
        buf.push(' ');
        if &func.name != "main" {
            buf.push_str(&self.prefix(&func.name));
        } else {
            buf.push_str(&func.name);
        }
        buf.push('(');
        buf.push_str(
            func.args
                .iter()
                .map(|arg| {
                    format!(
                        "{} {}",
                        self.emit_type(env, &arg.ty),
                        self.prefix(&arg.ident)
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        buf.push(')');
        buf.push('{');

        if let Some(ref body) = func.body {
            let env = &mut env.clone();
            Analyzer::collect_declarations(&body.nodes, env).unwrap();

            for node in body.nodes.iter() {
                self.emit_node(buf, env, node);
            }
            if &func.name == "main"
                && !matches!(body.nodes.last().unwrap(), Node::Stmnt(Stmnt::Ret(_)))
            {
                buf.push_str("return 0;");
            }
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

        fs::create_dir_all(&opts.outdir).into_diagnostic()?;
        fs::write(&tmp_src_path, src).unwrap();
        fs::write(&hash_path, format!("{program_hash:?}")).unwrap();

        // let include_arg = format!("-I{CORE_INCLUDE_PATH}");
        let mut args = vec![
            tmp_src_path.to_str().unwrap(),
            "-o",
            out_path.to_str().expect("valid out path"),
        ];

        if cfg!(debug_assertions) {
            args.extend_from_slice(&["-Wall", "-Wextra"]);
        }

        if opts.release {
            args.extend_from_slice(&["-O3", "-flto"]);
        }

        println!("{}", args.join(" "));

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
