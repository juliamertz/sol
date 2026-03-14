use std::borrow::Cow;
use std::fs;
use std::hash::Hasher;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;
use wyhash2::WyHash;

use crate::BuildOpts;
use crate::ast::{
    BinOp, Block, CallExpr, Expr, Fn, Ident, LiteralKind, MemberAccess, Node, NodeId, Op, OpKind,
    PrefixExpr, Stmnt,
};
use crate::codegen::{Compiler, Emitter, quote};
use crate::lexer::source::Span;
use crate::type_checker::ty::{IntTy, Type, UIntTy};
use crate::type_checker::TypeEnv;

const GC_HEADERS: &str = include_str!("include/gc.h");
const LIST_HEADERS: &str = include_str!("include/list.h");

#[derive(Debug, Error, Diagnostic)]
#[diagnostic(code(codegen::c))]
pub enum CodegenError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = CodegenError> = core::result::Result<T, E>;

#[derive(Default)]
struct InsertMarker {
    pos: Option<usize>,
    emit: String,
}

pub struct C {
    node_marker: InsertMarker,
    tempdir: PathBuf,
}

impl Default for C {
    fn default() -> Self {
        Self {
            node_marker: Default::default(),
            tempdir: std::env::temp_dir(),
        }
    }
}

impl Emitter for C {
    type Input = Vec<Node>;

    fn emit(&mut self, env: TypeEnv, ast: &Self::Input) -> String {
        let mut buf = String::new();

        let includes = [("gc.h", GC_HEADERS), ("list.h", LIST_HEADERS)];

        for (filename, contents) in includes {
            let path = self.tempdir.join(filename);
            fs::write(&path, contents).unwrap();
            buf.push_str(&format!("#include \"{}\"\n", path.to_str().unwrap()));
        }

        for node in ast {
            self.emit_node(&mut buf, &env, node);
        }

        buf
    }
}

impl C {
    // namespace prefix to be used for identifiers
    fn prefix(&self, ident: &Ident) -> String {
        format!(
            "__{prefix}_{ident}",
            prefix = env!("CARGO_PKG_NAME"),
            ident = ident.as_str()
        )
    }

    fn emit_op(&mut self, buf: &mut String, op: &Op) {
        let text = match op.kind {
            OpKind::Eq => "==",
            OpKind::Add => "+",
            OpKind::Sub => "-",
            OpKind::Mul => "*",
            OpKind::Div => "/",
            OpKind::Lt => "<",
            OpKind::Gt => ">",
            OpKind::And => "&&",
            OpKind::Or => "||",
        };
        buf.push_str(text);
    }

    fn emit_binop_expr(&mut self, buf: &mut String, env: &TypeEnv, binop: &BinOp) {
        self.emit_expr(buf, env, binop.lhs.as_ref());
        self.emit_op(buf, &binop.op);
        self.emit_expr(buf, env, binop.rhs.as_ref());
    }

    fn emit_call_expr(&mut self, buf: &mut String, env: &TypeEnv, call_expr: &CallExpr) {
        self.emit_expr(buf, env, &call_expr.func);

        buf.push('(');

        for (idx, arg) in call_expr.params.iter().enumerate() {
            self.emit_expr(buf, env, arg);
            if idx != call_expr.params.len() - 1 {
                buf.push(',');
            }
        }

        buf.push(')');
    }

    fn emit_node(&mut self, buf: &mut String, env: &TypeEnv, node: &Node) {
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

    fn emit_type(&mut self, _env: &TypeEnv, ty: impl Into<Type>) -> String {
        match ty.into() {
            Type::Int(kind) => match kind {
                IntTy::I8 => "int8_t",
                IntTy::I16 => "int16_t",
                IntTy::I32 => "int32_t",
                IntTy::I64 => "int64_t",
            },
            Type::UInt(kind) => match kind {
                UIntTy::U8 => "uint8_t",
                UIntTy::U16 => "uint16_t",
                UIntTy::U32 => "uint32_t",
                UIntTy::U64 => "uint64_t",
            },
            Type::Str => "char *",
            Type::Bool => "bool",
            Type::List(_) => "List",
            Type::Struct { ref ident, .. } => ident.as_str(),
            Type::Var(ref name) => name.as_str(),
            Type::Ptr(_ty) => todo!(),
            Type::Fn { .. } | Type::None => todo!(),
        }
        .into()
    }

    fn emit_block(&mut self, buf: &mut String, env: &TypeEnv, block: &Block) {
        for node in block.nodes.iter() {
            self.emit_node(buf, env, node);
        }
    }

    fn emit_ident(&mut self, buf: &mut String, env: &TypeEnv, ident: &Ident) {
        let ty = env.type_of(&ident.id);
        let ident = if let Some(Type::Fn {
            is_extern: true, ..
        }) = ty
        {
            ident.as_str()
        } else {
            &self.prefix(ident)
        };
        buf.push_str(ident);
    }

    fn emit_expr(&mut self, buf: &mut String, env: &TypeEnv, expr: &Expr) {
        dbg!(&expr);
        let ty = env.type_of(&expr.id()).unwrap();
        match expr {
            Expr::Ident(ident) => self.emit_ident(buf, env, ident),
            Expr::RawIdent(ident) => buf.push_str(ident.as_ref()),
            Expr::Literal(literal) => match &literal.kind {
                LiteralKind::Str(str) => buf.push_str(&quote(str)),
                LiteralKind::Int(int) => buf.push_str(&int.to_string()),
            },
            Expr::Prefix(PrefixExpr { op, rhs, .. }) => {
                self.emit_op(buf, op);
                self.emit_expr(buf, env, rhs);
            }
            Expr::BinOp(binop) => self.emit_binop_expr(buf, env, binop),
            Expr::Call(call_expr) => self.emit_call_expr(buf, env, call_expr),
            Expr::IfElse(r#if) => {
                buf.push_str("if(");
                self.emit_expr(buf, env, &r#if.condition);
                buf.push_str("){");
                for node in r#if.consequence.nodes.iter() {
                    self.emit_node(buf, env, node);
                }
                buf.push('}');
                if let Some(ref alternative) = r#if.alternative {
                    buf.push_str("else{");
                    for node in alternative.nodes.iter() {
                        self.emit_node(buf, env, node);
                    }
                    buf.push('}');
                }
            }

            Expr::Constructor(constructor) => {
                buf.push('(');
                buf.push_str(constructor.ident.as_str());
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

            Expr::MemberAccess(MemberAccess { lhs, ident, .. }) => {
                self.emit_expr(buf, env, lhs);
                buf.push('.');
                self.emit_ident(buf, env, ident);
            }

            Expr::Block(block) => self.emit_block(buf, env, block),

            Expr::List(list) => {
                let tmp_name = "inner"; // TODO: implement some system to avoid naming conflicts
                buf.push_str("({");
                buf.push_str("List");
                buf.push(' ');
                buf.push_str(tmp_name);
                buf.push('=');
                buf.push_str("list_alloc");

                for item in list.items.iter() {
                    self.emit_expr(
                        buf,
                        env,
                        &Expr::Call(CallExpr {
                            id: NodeId::DUMMY,
                            span: Span::default(),
                            func: Arc::from(Expr::RawIdent("list_push_rval".into())),
                            params: vec![
                                Expr::Ref(Arc::from(Expr::RawIdent(tmp_name.into()))),
                                item.clone(),
                            ]
                            .into(),
                        }),
                    );
                    self.node_marker.emit.push_str(buf);
                    self.node_marker.emit.push(';');
                }

                buf.push_str(tmp_name);
                buf.push(';');

                buf.push_str("})");
            }

            Expr::Ref(inner) => {
                buf.push('&');
                self.emit_expr(buf, env, inner);
            }

            Expr::Index(expr) => {
                buf.push_str("list_get_deref(");
                self.emit_expr(buf, env, &expr.expr);
                buf.push(',');
                buf.push_str(&self.emit_type(env, ty.to_owned()));
                buf.push(',');
                self.emit_expr(buf, env, &expr.idx);
                buf.push(')');
            }
        };
    }

    fn emit_stmnt(&mut self, buf: &mut String, env: &TypeEnv, stmnt: &Stmnt) {
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
                dbg!(&binding.val, &env);
                let ty = env.type_of(&binding.val.id()).unwrap();

                // TODO: pull out into seperate function
                if let Expr::List(list) = &binding.val {
                    for item in list.items.iter() {
                        let mut buf = String::new();
                        self.emit_expr(
                            &mut buf,
                            env,
                            &Expr::Call(CallExpr {
                                id: NodeId::DUMMY,
                                span: Span::default(),
                                func: Arc::from(Expr::RawIdent("list_push_rval".into())),
                                params: vec![
                                    Expr::Ref(Arc::from(Expr::Ident(binding.ident.clone()))),
                                    item.clone(),
                                ]
                                .into(),
                            }),
                        );
                        self.node_marker.emit.push_str(&buf);
                        self.node_marker.emit.push(';');
                    }

                    return;
                }

                buf.push_str(self.emit_type(env, ty.clone()).as_str());
                buf.push(' ');
                buf.push_str(&self.prefix(&binding.ident));
                buf.push('=');
                self.emit_expr(buf, env, &binding.val);
                buf.push(';');
            }
            Stmnt::StructDef(strct) => {
                buf.push_str("typedef struct ");
                buf.push_str(strct.ident.as_str());
                buf.push('{');
                for (ident, ty) in strct.fields.iter() {
                    buf.push_str(&self.emit_type(env, ty));
                    buf.push(' ');
                    buf.push_str(&self.prefix(ident));
                    buf.push(';');
                }
                buf.push('}');
                buf.push_str(strct.ident.as_str());
                buf.push(';');
            }
            Stmnt::Impl(_) => {
                todo!()
            }
        }
    }

    fn emit_fn(&mut self, buf: &mut String, env: &TypeEnv, func: &Fn) {
        if func.is_extern {
            return;
        }

        buf.push_str(&self.emit_type(env, &func.return_ty));
        buf.push(' ');
        if func.ident.as_str() != "main" {
            buf.push_str(&self.prefix(&func.ident));
        } else {
            buf.push_str(func.ident.as_str());
        }
        buf.push('(');
        buf.push_str(
            func.params
                .iter()
                .map(|(ident, ty)| format!("{} {}", self.emit_type(env, ty), self.prefix(ident)))
                .collect::<Vec<_>>()
                .join(",")
                .as_str(),
        );
        buf.push(')');
        buf.push('{');

        if let Some(ref body) = func.body {
            self.emit_block(buf, env, body);
            if func.ident.as_str() == "main"
                && !matches!(body.nodes.last().unwrap(), Node::Stmnt(Stmnt::Ret(_)))
            {
                buf.push_str("return 0;");
            }
        }

        buf.push('}');
    }
}

impl Compiler for C {
    type Err = CodegenError;

    fn build_exe(&self, source: &str, program: &str, opts: &BuildOpts) -> Result<PathBuf> {
        let mut hasher = WyHash::with_seed(0);
        hasher.write(program.as_bytes());
        let program_hash = hasher.finish();

        let out_path = opts.outdir.join(program);
        let hash_path = opts.outdir.join("hash");
        let tmp_src_path = opts.outdir.join("source.c");

        if let Ok(hash) = fs::read_to_string(&hash_path)
            && hash == format!("{program_hash:x}")
        {
            // TODO: verify hash of output binary, don't just assume it's right
            return Ok(out_path);
        };

        fs::create_dir_all(&opts.outdir)?;
        let src = if cfg!(debug_assertions) {
            self.format(source)
        } else {
            Cow::Borrowed(source)
        };
        fs::write(&tmp_src_path, src.as_bytes()).unwrap();
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

    fn format<'src>(&self, source: &'src str) -> Cow<'src, str> {
        let Ok(mut proc) = std::process::Command::new("clang-format")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        else {
            return Cow::Borrowed(source);
        };

        let mut stdin = proc.stdin.take().unwrap();
        let Ok(_) = stdin.write_all(source.as_bytes()) else {
            return Cow::Borrowed(source);
        };
        drop(stdin);

        let output = proc.wait_with_output().unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();

        Cow::Owned(stdout)
    }
}
