use std::fmt::{self, Display};

use crate::ast::*;
use crate::lexer::source::Span;

impl Display for IntTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
        })
    }
}

impl Display for UIntTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UIntTy::U8 => "u8",
            UIntTy::U16 => "u16",
            UIntTy::U32 => "u32",
            UIntTy::U64 => "u64",
        })
    }
}

pub struct PrettyPrinter {
    line_starts: Vec<usize>,
}

struct Pos {
    row: usize,
    col: usize,
}

impl PrettyPrinter {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    fn pos(&self, offset: usize) -> Pos {
        let row = self.line_starts.partition_point(|&s| s <= offset).saturating_sub(1);
        let col = offset - self.line_starts[row];
        Pos { row, col }
    }

    fn fmt_span(&self, f: &mut fmt::Formatter<'_>, span: Span) -> fmt::Result {
        let start = self.pos(span.offset());
        let end = self.pos(span.offset() + span.len());
        write!(f, "[{}:{} - {}:{}]", start.row, start.col, end.row, end.col)
    }

    fn indent(f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        for _ in 0..depth {
            f.write_str("  ")?;
        }
        Ok(())
    }

    fn fmt_ty(&self, f: &mut fmt::Formatter<'_>, ty: &Ty, depth: usize) -> fmt::Result {
        Self::indent(f, depth)?;
        write!(f, "(type ")?;
        self.fmt_span(f, ty.span)?;
        match &ty.kind {
            TyKind::Int(t) => write!(f, " {t}")?,
            TyKind::UInt(t) => write!(f, " {t}")?,
            TyKind::Bool => write!(f, " bool")?,
            TyKind::Str => write!(f, " str")?,
            TyKind::List { inner, size } => {
                match size {
                    Some(n) => write!(f, " list[{n}]")?,
                    None => write!(f, " list")?,
                }
                writeln!(f)?;
                self.fmt_ty(f, inner, depth + 1)?;
            }
            TyKind::Fn { params, returns, is_extern } => {
                if *is_extern {
                    write!(f, " extern fn")?;
                } else {
                    write!(f, " fn")?;
                }
                for param in params.iter() {
                    writeln!(f)?;
                    self.fmt_ty(f, param, depth + 1)?;
                }
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                write!(f, "(returns")?;
                writeln!(f)?;
                self.fmt_ty(f, returns, depth + 2)?;
                write!(f, ")")?;
            }
            TyKind::Var(ident) => write!(f, " {ident}")?,
        }
        write!(f, ")")
    }

    fn fmt_expr(&self, f: &mut fmt::Formatter<'_>, expr: &Expr, depth: usize) -> fmt::Result {
        match expr {
            Expr::Ident(ident) => {
                Self::indent(f, depth)?;
                write!(f, "(identifier ")?;
                self.fmt_span(f, ident.span)?;
                write!(f, " \"{}\")", ident.inner)
            }
            Expr::Literal(lit) => {
                Self::indent(f, depth)?;
                write!(f, "(literal ")?;
                self.fmt_span(f, lit.span)?;
                match &lit.kind {
                    LiteralKind::Str(s) => write!(f, " \"{}\")", s),
                    LiteralKind::Int(v) => write!(f, " {v})"),
                    LiteralKind::Bool(v) => write!(f, " {v})"),
                }
            }
            Expr::Block(block) => self.fmt_block(f, block, depth),
            Expr::BinOp(bin_op) => {
                Self::indent(f, depth)?;
                write!(f, "(binary_op ")?;
                self.fmt_span(f, bin_op.span)?;
                writeln!(f, " {}", fmt_binop(&bin_op.op.kind))?;
                self.fmt_expr(f, &bin_op.lhs, depth + 1)?;
                writeln!(f)?;
                self.fmt_expr(f, &bin_op.rhs, depth + 1)?;
                write!(f, ")")
            }
            Expr::Unary(unary) => {
                Self::indent(f, depth)?;
                write!(f, "(unary_op ")?;
                self.fmt_span(f, unary.span)?;
                writeln!(f, " {}", fmt_unaryop(&unary.op.kind))?;
                self.fmt_expr(f, &unary.rhs, depth + 1)?;
                write!(f, ")")
            }
            Expr::Call(call) => {
                Self::indent(f, depth)?;
                write!(f, "(call ")?;
                self.fmt_span(f, call.span)?;
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                writeln!(f, "func:")?;
                self.fmt_expr(f, &call.func, depth + 2)?;
                if !call.params.is_empty() {
                    writeln!(f)?;
                    Self::indent(f, depth + 1)?;
                    write!(f, "args:")?;
                    for param in call.params.iter() {
                        writeln!(f)?;
                        self.fmt_expr(f, param, depth + 2)?;
                    }
                }
                write!(f, ")")
            }
            Expr::Index(index) => {
                Self::indent(f, depth)?;
                write!(f, "(index ")?;
                self.fmt_span(f, index.span)?;
                writeln!(f)?;
                self.fmt_expr(f, &index.expr, depth + 1)?;
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                writeln!(f, "at:")?;
                self.fmt_expr(f, &index.idx, depth + 2)?;
                write!(f, ")")
            }
            Expr::IfElse(if_else) => {
                Self::indent(f, depth)?;
                write!(f, "(if_else ")?;
                self.fmt_span(f, if_else.span)?;
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                writeln!(f, "condition:")?;
                self.fmt_expr(f, &if_else.condition, depth + 2)?;
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                writeln!(f, "consequence:")?;
                self.fmt_block(f, &if_else.consequence, depth + 2)?;
                if let Some(alt) = &if_else.alternative {
                    writeln!(f)?;
                    Self::indent(f, depth + 1)?;
                    writeln!(f, "alternative:")?;
                    self.fmt_block(f, alt, depth + 2)?;
                }
                write!(f, ")")
            }
            Expr::List(list) => {
                Self::indent(f, depth)?;
                write!(f, "(list ")?;
                self.fmt_span(f, list.span)?;
                for item in list.items.iter() {
                    writeln!(f)?;
                    self.fmt_expr(f, item, depth + 1)?;
                }
                write!(f, ")")
            }
            Expr::Constructor(ctor) => {
                Self::indent(f, depth)?;
                write!(f, "(constructor ")?;
                self.fmt_span(f, ctor.span)?;
                write!(f, " \"{}\"", ctor.ident)?;
                for (name, val) in ctor.fields.iter() {
                    writeln!(f)?;
                    Self::indent(f, depth + 1)?;
                    writeln!(f, "{}:", name)?;
                    self.fmt_expr(f, val, depth + 2)?;
                }
                write!(f, ")")
            }
            Expr::MemberAccess(access) => {
                Self::indent(f, depth)?;
                write!(f, "(member_access ")?;
                self.fmt_span(f, access.span)?;
                writeln!(f, " .{}", access.ident)?;
                self.fmt_expr(f, &access.lhs, depth + 1)?;
                write!(f, ")")
            }
            Expr::Ref(inner) => {
                Self::indent(f, depth)?;
                write!(f, "(ref ")?;
                self.fmt_span(f, inner.span())?;
                writeln!(f)?;
                self.fmt_expr(f, inner, depth + 1)?;
                write!(f, ")")
            }
        }
    }

    fn fmt_block(&self, f: &mut fmt::Formatter<'_>, block: &Block, depth: usize) -> fmt::Result {
        Self::indent(f, depth)?;
        write!(f, "(block ")?;
        self.fmt_span(f, block.span)?;
        for stmnt in block.nodes.iter() {
            writeln!(f)?;
            self.fmt_stmnt(f, stmnt, depth + 1)?;
        }
        write!(f, ")")
    }

    fn fmt_stmnt(&self, f: &mut fmt::Formatter<'_>, stmnt: &Stmnt, depth: usize) -> fmt::Result {
        match stmnt {
            Stmnt::Let(let_) => {
                Self::indent(f, depth)?;
                write!(f, "(let ")?;
                self.fmt_span(f, let_.span)?;
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                write!(f, "name: \"{}\"", let_.ident)?;
                if let Some(ty) = &let_.ty {
                    writeln!(f)?;
                    Self::indent(f, depth + 1)?;
                    writeln!(f, "type:")?;
                    self.fmt_ty(f, ty, depth + 2)?;
                }
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                writeln!(f, "value:")?;
                self.fmt_expr(f, &let_.val, depth + 2)?;
                write!(f, ")")
            }
            Stmnt::Ret(ret) => {
                Self::indent(f, depth)?;
                write!(f, "(return ")?;
                self.fmt_span(f, ret.span)?;
                writeln!(f)?;
                self.fmt_expr(f, &ret.val, depth + 1)?;
                write!(f, ")")
            }
            Stmnt::Expr(expr) => self.fmt_expr(f, expr, depth),
        }
    }

    fn fmt_fn(&self, f: &mut fmt::Formatter<'_>, func: &Fn, depth: usize) -> fmt::Result {
        Self::indent(f, depth)?;
        match &func.kind {
            FnKind::Local { params, body } => {
                write!(f, "(function ")?;
                self.fmt_span(f, func.span)?;
                writeln!(f, " \"{}\"", func.ident)?;
                if !params.is_empty() {
                    Self::indent(f, depth + 1)?;
                    write!(f, "params:")?;
                    for (ident, ty) in params.iter() {
                        writeln!(f)?;
                        Self::indent(f, depth + 2)?;
                        write!(f, "(param ")?;
                        self.fmt_span(f, ident.span)?;
                        writeln!(f, " \"{}\"", ident)?;
                        self.fmt_ty(f, ty, depth + 3)?;
                        write!(f, ")")?;
                    }
                    writeln!(f)?;
                }
                Self::indent(f, depth + 1)?;
                writeln!(f, "returns:")?;
                self.fmt_ty(f, &func.return_ty, depth + 2)?;
                writeln!(f)?;
                Self::indent(f, depth + 1)?;
                writeln!(f, "body:")?;
                self.fmt_block(f, body, depth + 2)?;
                write!(f, ")")
            }
            FnKind::Extern { params, is_variadic } => {
                write!(f, "(extern_function ")?;
                self.fmt_span(f, func.span)?;
                writeln!(f, " \"{}\"", func.ident)?;
                if !params.is_empty() || *is_variadic {
                    Self::indent(f, depth + 1)?;
                    write!(f, "params:")?;
                    for (name, ty) in params.iter() {
                        writeln!(f)?;
                        Self::indent(f, depth + 2)?;
                        write!(f, "(param ")?;
                        self.fmt_span(f, name.span)?;
                        writeln!(f, " \"{}\"", name.inner)?;
                        self.fmt_ty(f, ty, depth + 3)?;
                        write!(f, ")")?;
                    }
                    if *is_variadic {
                        writeln!(f)?;
                        Self::indent(f, depth + 2)?;
                        write!(f, "(variadic)")?;
                    }
                    writeln!(f)?;
                }
                Self::indent(f, depth + 1)?;
                writeln!(f, "returns:")?;
                self.fmt_ty(f, &func.return_ty, depth + 2)?;
                write!(f, ")")
            }
        }
    }

    fn fmt_item(&self, f: &mut fmt::Formatter<'_>, item: &Item, depth: usize) -> fmt::Result {
        match item {
            Item::Use(use_) => {
                Self::indent(f, depth)?;
                write!(f, "(use ")?;
                self.fmt_span(f, use_.span)?;
                if use_.is_extern {
                    write!(f, " extern")?;
                }
                write!(f, " \"{}\")", use_.name.inner)
            }
            Item::Fn(func) => self.fmt_fn(f, func, depth),
            Item::StructDef(def) => {
                Self::indent(f, depth)?;
                write!(f, "(struct_def ")?;
                self.fmt_span(f, def.span)?;
                write!(f, " \"{}\"", def.ident)?;
                for (name, ty) in def.fields.iter() {
                    writeln!(f)?;
                    Self::indent(f, depth + 1)?;
                    write!(f, "(field ")?;
                    self.fmt_span(f, name.span)?;
                    writeln!(f, " \"{}\"", name.inner)?;
                    self.fmt_ty(f, ty, depth + 2)?;
                    write!(f, ")")?;
                }
                write!(f, ")")
            }
            Item::Impl(impl_) => {
                Self::indent(f, depth)?;
                write!(f, "(impl ")?;
                self.fmt_span(f, impl_.span)?;
                write!(f, " \"{}\"", impl_.ident)?;
                for func in impl_.items.iter() {
                    writeln!(f)?;
                    self.fmt_fn(f, func, depth + 1)?;
                }
                write!(f, ")")
            }
        }
    }

    pub fn fmt_module(&self, f: &mut fmt::Formatter<'_>, module: &Module) -> fmt::Result {
        write!(f, "(module")?;
        for item in module.items.iter() {
            writeln!(f)?;
            self.fmt_item(f, item, 1)?;
        }
        writeln!(f, ")")
    }
}

pub struct DisplayModule<'a> {
    pub module: &'a Module,
    pub source: &'a str,
}

impl Display for DisplayModule<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printer = PrettyPrinter::new(self.source);
        printer.fmt_module(f, self.module)
    }
}

fn fmt_binop(op: &BinOpKind) -> &'static str {
    match op {
        BinOpKind::Eq => "==",
        BinOpKind::Add => "+",
        BinOpKind::Sub => "-",
        BinOpKind::Mul => "*",
        BinOpKind::Div => "/",
        BinOpKind::Lt => "<",
        BinOpKind::Gt => ">",
        BinOpKind::And => "and",
        BinOpKind::Or => "or",
    }
}

fn fmt_unaryop(op: &UnaryOpKind) -> &'static str {
    match op {
        UnaryOpKind::Negate => "-",
        UnaryOpKind::Not => "not",
    }
}
