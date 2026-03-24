use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::{BinOpKind, LiteralKind, UnaryOpKind};
use crate::codegen::c::ast::*;
use crate::codegen::c::compiler::{CcOpts, cc};
use crate::codegen::{BuildOpts, Compiler, Emitter};
use crate::hir;
use crate::type_checker::ty::{IntTy, Type, UIntTy};
use crate::type_checker::{TypeEnv, TypeId};

pub mod ast;
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

pub struct C {
    tempdir: PathBuf,
}

impl Default for C {
    fn default() -> Self {
        Self {
            tempdir: std::env::temp_dir(),
        }
    }
}

impl Emitter for C {
    fn emit(&mut self, env: TypeEnv, hir: &hir::Module<'_>) -> String {
        let mut tu = CTranslationUnit::default();

        let includes = [("gc.h", GC_HEADERS), ("list.h", LIST_HEADERS)];
        for (filename, contents) in includes {
            let path = self.tempdir.join(filename);
            fs::write(&path, contents).unwrap();
            tu.push(CItem::include_local(path.to_str().unwrap()));
        }

        for item in hir.items.iter() {
            if let Some(c_item) = self.lower_item(&env, item) {
                tu.push(c_item);
            }
        }

        tu.to_string()
    }
}

impl C {
    fn unary(&self, ident: &hir::Ident) -> String {
        format!(
            "__{unary}_{ident}",
            unary = env!("CARGO_PKG_NAME"),
            ident = ident.as_str()
        )
    }

    fn lower_type(&self, env: &TypeEnv, type_id: &TypeId) -> CType {
        let ty = env.types.get(type_id).expect("type to be defined");
        match ty {
            Type::Int(kind) => match kind {
                IntTy::I8 => CType::Int8,
                IntTy::I16 => CType::Int16,
                IntTy::I32 => CType::Int32,
                IntTy::I64 => CType::Int64,
            },
            Type::UInt(kind) => match kind {
                UIntTy::U8 => CType::UInt8,
                UIntTy::U16 => CType::UInt16,
                UIntTy::U32 => CType::UInt32,
                UIntTy::U64 => CType::UInt64,
            },
            Type::Str => CType::CharPtr,
            Type::Bool => CType::Bool,
            Type::List(..) => CType::named("List"),
            Type::Struct { name, .. } => CType::named(&*name.inner),
            Type::Ptr(_) => todo!(),
            Type::Fn { .. } => todo!(),
            Type::Unit => todo!(),
        }
    }

    fn lower_bin_op(kind: &BinOpKind) -> &'static str {
        match kind {
            BinOpKind::Eq => "==",
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Lt => "<",
            BinOpKind::Gt => ">",
            BinOpKind::And => "&&",
            BinOpKind::Or => "||",
        }
    }

    fn lower_unary_op(kind: &UnaryOpKind) -> &'static str {
        match kind {
            UnaryOpKind::Negate => "-",
            UnaryOpKind::Not => "!",
        }
    }

    fn lower_ident(&self, env: &TypeEnv, ident: &hir::Ident) -> CExpr {
        let ty = env.types.get(&ident.ty);
        let name = if let Some(Type::Fn {
            is_extern: true, ..
        }) = ty
        {
            ident.as_str().to_owned()
        } else {
            self.unary(ident)
        };
        CExpr::ident(name)
    }

    fn lower_expr(&self, env: &TypeEnv, expr: &hir::Expr) -> CExpr {
        match expr {
            hir::Expr::Ident(ident) => self.lower_ident(env, ident),

            hir::Expr::Literal(literal) => match &literal.kind {
                LiteralKind::Str(str) => CExpr::str(str.as_ref()),
                LiteralKind::Int(int) => CExpr::int(*int),
                LiteralKind::Bool(val) => CExpr::bool(*val),
            },

            hir::Expr::Unary(unary) => {
                CExpr::unary(Self::lower_unary_op(&unary.op.kind), self.lower_expr(env, &unary.rhs))
            }

            hir::Expr::BinOp(binop) => self
                .lower_expr(env, &binop.lhs)
                .binop(Self::lower_bin_op(&binop.op.kind), self.lower_expr(env, &binop.rhs)),

            hir::Expr::Call(call) => CExpr::call(
                self.lower_expr(env, &call.func),
                call.params.iter().map(|p| self.lower_expr(env, p)).collect(),
            ),

            hir::Expr::IfElse(if_else) => {
                let cond = self.lower_expr(env, &if_else.condition);
                let then = self.lower_block(env, &if_else.consequence);
                let else_ = if_else
                    .alternative
                    .as_ref()
                    .map(|alt| self.lower_block(env, alt));
                let stmts = vec![match else_ {
                    Some(else_stmts) => CStmt::if_else(cond, then, else_stmts),
                    None => CStmt::if_(cond, then),
                }];

                CExpr::stmt_expr(stmts)
            }

            hir::Expr::Constructor(ctor) => CExpr::compound_lit(
                ctor.ident.as_str(),
                ctor.fields
                    .iter()
                    .map(|(ident, expr)| (self.unary(ident), self.lower_expr(env, expr)))
                    .collect(),
            ),

            hir::Expr::MemberAccess(access) => self
                .lower_expr(env, &access.lhs)
                .member(self.lower_ident(env, &access.ident).to_string()),

            hir::Expr::Block(block) => {
                let stmts = self.lower_block(env, block);
                CExpr::stmt_expr(stmts)
            }

            hir::Expr::List(list) => {
                let tmp_name = "inner"; // TODO: naming conflict avoidance
                let elem_ty = self.lower_type(env, &list.ty);

                let mut stmts = vec![CStmt::var(
                    CType::named("List"),
                    tmp_name,
                    CExpr::call(
                        CExpr::ident("list_alloc"),
                        vec![CExpr::sizeof(elem_ty), CExpr::int(8)], // TODO: smart sizing
                    ),
                )];

                for item in list.items.iter() {
                    stmts.push(CStmt::expr(CExpr::call(
                        CExpr::ident("list_push_rval"),
                        vec![CExpr::ident(tmp_name).addr_of(), self.lower_expr(env, item)],
                    )));
                }

                stmts.push(CStmt::expr(CExpr::ident(tmp_name)));
                CExpr::stmt_expr(stmts)
            }

            hir::Expr::Ref(inner) => self.lower_expr(env, inner).addr_of(),

            hir::Expr::Index(idx) => CExpr::call(
                CExpr::ident("list_get_deref"),
                vec![
                    self.lower_expr(env, &idx.expr),
                    CExpr::ident(self.lower_type(env, &idx.ty).to_string()),
                    self.lower_expr(env, &idx.idx),
                ],
            ),
        }
    }

    fn lower_stmnt(&self, env: &TypeEnv, stmnt: &hir::Stmnt) -> CStmt {
        match stmnt {
            hir::Stmnt::Let(binding) => {
                let ty = self.lower_type(env, binding.val.type_id());
                CStmt::var(ty, self.unary(&binding.ident), self.lower_expr(env, &binding.val))
            }
            hir::Stmnt::Ret(ret) => CStmt::ret(self.lower_expr(env, &ret.val)),
            hir::Stmnt::Expr(expr) => CStmt::expr(self.lower_expr(env, expr)),
        }
    }

    fn lower_block(&self, env: &TypeEnv, block: &hir::Block) -> Vec<CStmt> {
        block.nodes.iter().map(|s| self.lower_stmnt(env, s)).collect()
    }

    fn lower_item(&self, env: &TypeEnv, item: &hir::Item) -> Option<CItem> {
        match item {
            hir::Item::Fn(func) => {
                if func.is_extern {
                    return None;
                }

                let ret = self.lower_type(env, &func.return_ty);
                let name = if func.ident.as_str() == "main" {
                    "main".to_owned()
                } else {
                    self.unary(&func.ident)
                };

                let params: Vec<_> = func
                    .params
                    .iter()
                    .map(|(ident, ty)| (self.lower_type(env, ty), self.unary(ident)))
                    .collect();

                let mut body = func
                    .body
                    .as_ref()
                    .map(|b| self.lower_block(env, b))
                    .unwrap_or_default();

                if func.ident.as_str() == "main" {
                    let needs_return = !matches!(body.last(), Some(CStmt::Return(_)));
                    if needs_return {
                        body.push(CStmt::ret(CExpr::int(0)));
                    }
                }

                Some(CItem::fn_def(ret, name, params, body))
            }

            hir::Item::Use(r#use) => {
                assert!(
                    r#use.is_extern,
                    "non extern `use` statements are not supported yet"
                );
                Some(CItem::include_system(format!("{}.h", r#use.ident.inner)))
            }

            hir::Item::StructDef(strct) => {
                let fields = strct
                    .fields
                    .iter()
                    .map(|(ident, ty)| (self.lower_type(env, ty), self.unary(ident)))
                    .collect();
                Some(CItem::typedef_struct(strct.name.as_str(), fields))
            }
        }
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

        cc(&source_path, &cc_opts)?;

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
