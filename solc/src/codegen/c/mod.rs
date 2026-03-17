use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use miette::Diagnostic;
use thiserror::Error;

use crate::BuildOpts;
use crate::ast::{LiteralKind, Op, OpKind};
use crate::codegen::c::compiler::{CcOpts, cc};
use crate::codegen::{Compiler, Emitter, quote};
use crate::hir::{
    BinOp, Block, Call, Expr, Fn, Ident, List, MemberAccess, Module, Node, Prefix, Stmnt,
};
use crate::type_checker::ty::{IntTy, Type, UIntTy};
use crate::type_checker::{TypeEnv, TypeId};

mod compiler;

const GC_HEADERS: &str = include_str!("include/gc.h");
const LIST_HEADERS: &str = include_str!("include/list.h");

#[derive(Debug, Error, Diagnostic)]
pub enum CodegenError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("cc error: {0}")]
    #[diagnostic(transparent)]
    Cc(#[from] compiler::CcError),
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
    fn emit(&mut self, env: TypeEnv, hir: &Module<'_>) -> String {
        let mut buf = String::new();

        let includes = [("gc.h", GC_HEADERS), ("list.h", LIST_HEADERS)];

        for (filename, contents) in includes {
            let path = self.tempdir.join(filename);
            fs::write(&path, contents).unwrap();
            buf.push_str(&format!("#include \"{}\"\n", path.to_str().unwrap()));
        }

        for node in hir.nodes.iter() {
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
        self.emit_op(buf, binop.op);
        self.emit_expr(buf, env, binop.rhs.as_ref());
    }

    fn emit_expr_list(&mut self, buf: &mut String, env: &TypeEnv, list: &[Expr<'_>]) {
        for (idx, arg) in list.iter().enumerate() {
            self.emit_expr(buf, env, arg);
            if idx != list.len() - 1 {
                buf.push(',');
            }
        }
    }

    fn emit_call_expr(&mut self, buf: &mut String, env: &TypeEnv, call_expr: &Call) {
        self.emit_expr(buf, env, &call_expr.func);
        buf.push('(');
        self.emit_expr_list(buf, env, &call_expr.params);
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

    fn emit_type(&mut self, env: &TypeEnv, type_id: &TypeId) -> String {
        let ty = env.types.get(type_id).expect("type to be defined");
        match ty {
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
            Type::List(..) => "List",
            Type::Struct { ident, .. } => ident.as_str(),
            Type::Ptr(_) => todo!(),
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
        let ty = env.types.get(&ident.ty);
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

    fn emit_list_expr(&mut self, buf: &mut String, env: &TypeEnv, list: &List<'_>) {
        let tmp_name = "inner"; // TODO: implement some system to avoid naming conflicts
        buf.push_str("({");
        buf.push_str("List");
        buf.push(' ');
        buf.push_str(tmp_name);
        buf.push('=');
        buf.push_str("list_alloc");
        buf.push_str("(sizeof(");
        buf.push_str(&self.emit_type(env, &list.ty));
        buf.push(')');
        buf.push(',');
        buf.push_str("10"); // TODO: smart list sizing
        buf.push(')');
        buf.push(';');

        for item in list.items.iter() {
            buf.push_str("list_push_rval");
            buf.push('(');
            buf.push('&');
            buf.push_str(tmp_name);
            buf.push(',');
            self.emit_expr(buf, env, item);
            buf.push(')');
            buf.push(';');
        }

        buf.push_str(tmp_name);
        buf.push(';');

        buf.push_str("})");
    }

    fn emit_expr(&mut self, buf: &mut String, env: &TypeEnv, expr: &Expr) {
        match expr {
            Expr::Ident(ident) => self.emit_ident(buf, env, ident),
            // Expr::RawIdent(ident) => buf.push_str(ident.as_ref()),
            Expr::Literal(literal) => match &literal.kind {
                LiteralKind::Str(str) => buf.push_str(&quote(str)),
                LiteralKind::Int(int) => buf.push_str(&int.to_string()),
            },
            Expr::Prefix(Prefix { op, rhs, .. }) => {
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

            Expr::List(list) => self.emit_list_expr(buf, env, list),

            Expr::Ref(inner) => {
                buf.push('&');
                self.emit_expr(buf, env, inner);
            }

            Expr::Index(expr) => {
                buf.push_str("list_get_deref(");
                self.emit_expr(buf, env, &expr.expr);
                buf.push(',');
                buf.push_str(&self.emit_type(env, &expr.ty));
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
                assert!(
                    r#use.is_extern,
                    "non extern `use` statements are not supported yet"
                );
                buf.push_str(format!("#include <{}.h>\n", r#use.ident.inner).as_str());
            }
            Stmnt::Ret(ret) => {
                buf.push_str("return");
                buf.push(' ');
                self.emit_expr(buf, env, &ret.val);
                buf.push(';');
            }
            Stmnt::Let(binding) => {
                let type_id = binding.val.type_id();
                buf.push_str(self.emit_type(env, type_id).as_str());
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
        let out_path = opts.outdir.join(program);
        let source_path = opts.outdir.join("source.c");

        fs::create_dir_all(&opts.outdir)?;
        let src = if cfg!(debug_assertions) {
            self.format(source)
        } else {
            Cow::Borrowed(source)
        };
        fs::write(&source_path, src.as_bytes())?;

        let cc_opts = CcOpts::new(&out_path)
            .link_time_optimization(true)
            .warnings(if cfg!(debug_assertions) {
                vec!["all", "extra"]
            } else {
                vec![]
            });

        let _output = cc(&source_path, &cc_opts)?;

        if opts.cleanup {
            fs::remove_file(source_path)?;
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
