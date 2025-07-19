use crate::BuildOpts;
use crate::analyzer::{self, Analyzer, TypeEnv};
use crate::ast::{Block, CallExpr, Expr, Fn, InfixExpr, Node, Op, PrefixExpr, Stmnt};
use crate::codegen::{Compiler, Emitter};

use std::fs;
use std::hash::Hasher;
use std::path::PathBuf;

use miette::{IntoDiagnostic, Result};
use wyhash2::WyHash;

const CORE_INCLUDE_PATH: &str = "/Users/julia/projects/2025/sol/src/codegen/c/include";
const CORE_INCLUDES: &[&str] = &["gc.h", "list.h"];

#[derive(Default)]
struct InsertMarker {
    pos: Option<usize>,
    emit: String,
}

#[derive(Default)]
pub struct C {
    node_marker: InsertMarker,
    block_marker: InsertMarker,
}

impl Emitter for C {
    type Input = Vec<Node>;

    fn emit(&mut self, ast: &Self::Input, env: &mut TypeEnv) -> String {
        let mut buf = String::new();

        Analyzer::collect_declarations(ast, env).unwrap();

        // buf.push_str(include_str!("include/gc.h"));
        // buf.push_str(include_str!("include/list.h"));

        for file in CORE_INCLUDES {
            buf.push_str(&format!("#include \"{CORE_INCLUDE_PATH}/{file}\"\n"));
        }

        for node in ast {
            self.emit_node(&mut buf, env, node);
        }

        buf
    }
}

impl C {
    // namespace prefix to be used for all identifiers
    fn prefix(&self, ident: &str) -> String {
        format!("__{prefix}_{ident}", prefix = env!("CARGO_PKG_NAME"))
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
            Op::Chain => ".",
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
            Expr::RawIdent(ident) => ident,
            _ => todo!("{call_expr:?}"),
        };
        let declaration = env.get(name);

        if let Some(analyzer::Type::Fn {
            is_extern: true, ..
        }) = declaration
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
        self.node_marker.pos = Some(buf.len());

        match node {
            Node::Expr(expr) => {
                self.emit_expr(buf, env, expr);
                buf.push(';');
            }
            Node::Stmnt(stmnt) => self.emit_stmnt(buf, env, stmnt),
        }

        if let Some(pos) = self.node_marker.pos {
            buf.insert_str(pos, &self.node_marker.emit);
            self.node_marker = InsertMarker::default();
        }
    }

    fn emit_type(&mut self, _env: &mut TypeEnv, ty: impl Into<analyzer::Type>) -> String {
        match ty.into() {
            analyzer::Type::Int => "int",
            analyzer::Type::Str => "char *",
            analyzer::Type::Bool => "bool",
            analyzer::Type::List(_) => "List",
            analyzer::Type::Struct { ref ident, .. } => ident,
            analyzer::Type::Var(ref name) => name,
            analyzer::Type::Ptr(_ty) => todo!(),
            analyzer::Type::Fn { .. } | analyzer::Type::Any => todo!(),
        }
        .into()
    }

    fn emit_block(&mut self, buf: &mut String, env: &mut TypeEnv, block: &Block) {
        let env = &mut env.clone();
        let pre_define = Analyzer::collect_declarations(&block.nodes, env).unwrap();

        // TODO: refactor this into block_marker
        for (ident, ty) in pre_define.into_iter() {
            if let analyzer::Type::List((inner, _size)) = ty {
                buf.push_str(
                    format!(
                        "List {ident}=list_alloc(sizeof({ty}), 64);",
                        ident = self.prefix(&ident),
                        ty = self.emit_type(env, *inner)
                    )
                    .as_str(),
                );
            }
        }

        for node in &block.nodes {
            self.emit_node(buf, env, node);
        }
    }

    fn emit_expr(&mut self, buf: &mut String, env: &mut TypeEnv, expr: &Expr) {
        match expr {
            Expr::Ident(ident) => buf.push_str(&self.prefix(ident)),
            Expr::RawIdent(ident) => buf.push_str(ident),
            Expr::IntLit(val) => buf.push_str(&val.to_string()),
            Expr::StrLit(val) => buf.push_str(format!("\"{val}\"").as_str()),
            Expr::Prefix(prefix_expr) => todo!("prefix expr"),
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

            Expr::Constructor(constructor) => {
                buf.push('(');
                buf.push_str(&constructor.name);
                buf.push(')');
                buf.push('{');
                for (ident, expr) in constructor.fields.iter() {
                    buf.push('.');
                    buf.push_str(&self.prefix(ident));
                    buf.push('=');
                    self.emit_expr(buf, env, expr);
                    buf.push(',');
                }
                buf.push('}');
            }

            Expr::Block(block) => self.emit_block(buf, env, block),

            Expr::List(_) => unreachable!(),

            Expr::Ref(inner) => {
                buf.push('&');
                self.emit_expr(buf, env, inner);
            }

            Expr::Index(expr) => {
                let ty = Analyzer::check_expr(&Expr::Index(expr.clone()), env).unwrap();
                buf.push_str("list_get_deref(");
                self.emit_expr(buf, env, &expr.val);
                buf.push(',');
                buf.push_str(&self.emit_type(env, ty));
                buf.push(',');
                self.emit_expr(buf, env, &expr.idx);
                buf.push(')');
            }
        };
    }

    fn emit_stmnt(&mut self, buf: &mut String, env: &mut TypeEnv, stmnt: &Stmnt) {
        let ty = Analyzer::check_stmnt(stmnt, env).unwrap();

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
                // TODO: pull out into seperate function
                if let Expr::List(list) = &binding.val {
                    for item in list.items.clone() {
                        let mut buf = String::new();
                        self.emit_expr(
                            &mut buf,
                            env,
                            &Expr::Call(CallExpr {
                                func: Box::new(Expr::RawIdent("list_push_rval".into())),
                                args: vec![
                                    Expr::Ref(Box::new(Expr::Ident(binding.name.clone()))),
                                    item,
                                ],
                            }),
                        );
                        // Box::new(Expr::RawIdent("list_push_rval".into()))
                        self.node_marker.emit.push_str(&buf);
                        self.node_marker.emit.push(';');
                    }

                    return;
                }

                buf.push_str(self.emit_type(env, ty.clone()).as_str());
                buf.push(' ');
                buf.push_str(&self.prefix(&binding.name));
                buf.push('=');
                self.emit_expr(buf, env, &binding.val);
                buf.push(';');
            }
            Stmnt::StructDef(strct) => {
                buf.push_str("typedef struct ");
                buf.push_str(&strct.ident);
                buf.push('{');
                for (ident, ty) in strct.fields.iter() {
                    buf.push_str(&self.emit_type(env, ty));
                    buf.push(' ');
                    buf.push_str(&self.prefix(ident));
                    buf.push(';');
                }
                buf.push('}');
                buf.push_str(&strct.ident);
                buf.push(';');
            }
            Stmnt::Impl(_) => {
                todo!()
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
                .map(|(ident, ty)| format!("{} {}", self.emit_type(env, ty), self.prefix(ident)))
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        buf.push(')');
        buf.push('{');

        if let Some(ref body) = func.body {
            let env = &mut env.clone();
            self.emit_block(buf, env, body);
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
