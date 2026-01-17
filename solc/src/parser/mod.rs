// required for miette `Diagnostic` derive
// see: https://github.com/rust-lang/rust/issues/147648
#![allow(unused_assignments)]

use std::path::PathBuf;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::source::{SourceInfo, Span};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::ast::*;

#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(parser))]
pub enum ParseError {
    #[error("expected")]
    Expected {
        #[source_code]
        src: SourceInfo,

        #[label("this is of kind {actual} but was expected to be {expected}")]
        span: Span,

        expected: TokenKind,
        actual: TokenKind,

        #[help]
        help: Option<String>,
    },

    #[error("invalid operator")]
    InvalidOperator {
        #[source_code]
        src: SourceInfo,

        #[label("here")]
        span: Span,

        #[help]
        help: Option<String>,
    },

    #[error("unhandled token: {:?}", token.kind)]
    Todo {
        #[source_code]
        src: SourceInfo,

        token: Token,

        #[label("this token")]
        span: Span,
    },
}

pub type Result<T, E = ParseError> = core::result::Result<T, E>;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum Prec {
    #[default]
    Lowest,
    AndOr,     // && or || - lower precedence than equality
    Eq,        // ==
    Cmp,       // > or <
    Sum,       // +
    Product,   // *
    Prefix,    // -a, !a or &a
    Call,      // func()
    Construct, // Point { x : 10, y : 5 }
    // Index, // list[0]
    Chain, // mod.field
}

impl From<&Token> for Prec {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add | TokenKind::Sub => Self::Sum,
            TokenKind::Eq => Self::Eq,
            TokenKind::LParen => Self::Call,
            TokenKind::LSquirly => Self::Construct,
            TokenKind::LAngle | TokenKind::RAngle => Self::Cmp,
            TokenKind::Asterisk => Self::Product,
            TokenKind::And | TokenKind::Or => Self::AndOr,
            TokenKind::Dot => Self::Chain,
            TokenKind::Bang | TokenKind::Ampersand => Self::Prefix,
            _ => Self::Lowest,
        }
    }
}

#[derive(Default)]
pub struct Context {
    id: u32,
}

impl Context {
    fn next_id(&mut self) -> NodeId {
        let id = self.id;
        self.id += 1;
        NodeId::new(id)
    }
}

pub struct Parser {
    pub lex: Lexer,
    pub ctx: Context,
    pub tokens: Vec<Token>,
    pub curr: Token,
    pub next: Option<Token>,
}

impl Parser {
    pub fn new(file_path: PathBuf, content: impl ToString) -> Self {
        let mut lex = Lexer::new(file_path, content);
        let curr = lex
            .read_token()
            .unwrap_or(Token::new(TokenKind::Eof, "", lex.pos));
        let next = lex.read_token();
        let ctx = Context::default();
        Self {
            lex,
            ctx,
            curr,
            next,
            tokens: vec![],
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Node>> {
        let mut nodes = vec![];

        loop {
            if self.at(TokenKind::Eof) {
                break;
            }

            self.skip_whitespace();
            match self.node() {
                Ok(node) => nodes.push(node),
                Err(err) => return Err(err),
            }
        }

        Ok(nodes)
    }

    fn advance(&mut self) -> Option<Token> {
        let curr = self.next.clone();
        if let Some(next) = self.next.clone() {
            self.curr = next;
        }

        self.next = self.lex.read_token();
        self.tokens.push(self.next.clone()?);
        curr
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token> {
        if self.curr.kind != expected {
            return Err(ParseError::Expected {
                src: self.lex.source(),
                span: self.curr.span,
                expected,
                actual: self.curr.kind,
                help: None,
            });
        }
        Ok(self.curr.clone())
    }

    fn accept(&mut self, expected: TokenKind) -> Option<Token> {
        if self.at(expected) {
            let tok = self.curr.clone();
            self.advance();
            Some(tok)
        } else {
            None
        }
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token> {
        let tok = self.expect(expected)?;
        self.advance();
        Ok(tok)
    }

    fn at(&mut self, kind: TokenKind) -> bool {
        self.curr.kind == kind
    }

    fn skip_whitespace(&mut self) {
        while self.at(TokenKind::Newline) {
            self.advance();
        }
    }

    pub fn node(&mut self) -> Result<Node> {
        let node = if matches!(
            self.curr.kind,
            TokenKind::Ret
                | TokenKind::Use
                | TokenKind::Fn
                | TokenKind::Extern
                | TokenKind::Let
                | TokenKind::Struct
        ) {
            Node::Stmnt(self.stmnt()?)
        } else {
            Node::Expr(self.expr(Prec::default())?)
        };

        self.skip_whitespace();

        Ok(node)
    }

    fn block(&mut self) -> Result<Block> {
        let span = self.curr.span;
        let mut nodes = vec![];
        loop {
            if self.curr.kind.is_terminator() {
                break;
            }
            nodes.push(self.node()?);
        }

        let nodes = Arc::from(nodes);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&self.curr.span);
        Ok(Block { nodes, id, span })
    }

    fn ident(&mut self) -> Result<Ident> {
        let token = self.consume(TokenKind::Ident)?;
        let id = self.ctx.next_id();
        Ok(Ident {
            id,
            span: token.span,
            inner: Arc::from(token.text),
        })
    }

    fn ty(&mut self) -> Result<Ty> {
        let span = self.curr.span;
        let ident = self.ident()?;
        let kind = match ident.as_ref() {
            "i8" => TyKind::Int(IntTy::I8),
            "i16" => TyKind::Int(IntTy::I16),
            "i32" => TyKind::Int(IntTy::I32),
            "i64" => TyKind::Int(IntTy::I64),
            "u8" => TyKind::UInt(UIntTy::U8),
            "u16" => TyKind::UInt(UIntTy::U16),
            "u32" => TyKind::UInt(UIntTy::U32),
            "u64" => TyKind::UInt(UIntTy::U64),
            "Bool" => TyKind::Bool,
            "Str" => TyKind::Str,
            _ => TyKind::Var(ident),
        };
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&self.curr.span);
        let mut ty = Ty { kind, id, span };

        if self.at(TokenKind::LBracket) {
            self.consume(TokenKind::LBracket)?;
            self.consume(TokenKind::RBracket)?;
            let kind = TyKind::List {
                inner: Arc::from(ty),
                size: None,
            };
            let id = self.ctx.next_id();
            let span = span.enclosing_to(&self.curr.span);
            ty = Ty { kind, id, span }
        }

        Ok(ty)
    }

    fn func(&mut self) -> Result<Fn> {
        let span = self.curr.span;
        let is_extern = self.at(TokenKind::Extern);
        if is_extern {
            self.advance();
        }

        self.consume(TokenKind::Fn)?;
        let ident = self.ident()?;
        self.consume(TokenKind::LParen)?;
        let mut params = vec![];
        while self.curr.kind != TokenKind::RParen {
            params.push(self.typed_param()?);
            if self.at(TokenKind::Comma) {
                self.advance();
            }
        }
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::Arrow)?;
        let return_ty = self.ty()?;
        self.skip_whitespace();

        let body = if is_extern || self.curr.kind.is_terminator() {
            None
        } else {
            let span = self.curr.span;
            let mut nodes = vec![];
            while self.curr.kind != TokenKind::End {
                nodes.push(self.node()?);
            }

            self.consume(TokenKind::End)?;

            let nodes = Arc::from(nodes);
            let id = self.ctx.next_id();
            let span = span.enclosing_to(&self.curr.span);
            Some(Block { nodes, id, span })
        };

        let params = Arc::from(params);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&self.curr.span);
        Ok(Fn {
            is_extern,
            ident,
            params,
            return_ty,
            body,
            id,
            span,
        })
    }

    fn r#use(&mut self) -> Result<Use> {
        let span = self.curr.span;
        self.consume(TokenKind::Use)?;
        let ident = self.ident()?;
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&self.curr.span);
        Ok(Use { ident, id, span })
    }

    fn typed_param(&mut self) -> Result<(Ident, Ty)> {
        let ident = self.ident()?;
        self.consume(TokenKind::Colon)?;
        let ty = self.ty()?;
        Ok((ident, ty))
    }

    fn value_param(&mut self) -> Result<(Ident, Expr)> {
        let ident = self.ident()?;
        self.consume(TokenKind::Colon)?;
        let expr = self.expr(Prec::Lowest)?;
        if self.at(TokenKind::Comma) {
            self.advance();
        }

        Ok((ident, expr))
    }

    fn typed_params(&mut self) -> Result<Vec<(Ident, Ty)>> {
        let mut args = vec![];

        loop {
            self.skip_whitespace();
            if self.curr.kind.is_terminator() {
                break;
            }

            args.push(self.typed_param()?);
        }

        Ok(args)
    }

    fn value_params(&mut self) -> Result<Vec<(Ident, Expr)>> {
        let mut args = vec![];

        loop {
            self.skip_whitespace();
            if self.curr.kind.is_terminator() {
                break;
            }

            args.push(self.value_param()?);
        }

        Ok(args)
    }

    fn stmnt(&mut self) -> Result<Stmnt> {
        let stmnt = match self.curr.kind {
            TokenKind::Fn | TokenKind::Extern => Stmnt::Fn(self.func()?),
            TokenKind::Use => Stmnt::Use(self.r#use()?),
            TokenKind::Let => Stmnt::Let(self.r#let()?),
            TokenKind::Struct => Stmnt::StructDef(self.struct_def()?),
            TokenKind::Ret => {
                let span = self.curr.span;
                self.consume(TokenKind::Ret)?;
                let val = self.expr(Prec::default())?;
                let id = self.ctx.next_id();
                let span = span.enclosing_to(&self.curr.span);
                Stmnt::Ret(Ret { val, id, span })
            }
            _ => panic!("TODO: {}", self.curr.kind),
        };

        Ok(stmnt)
    }

    fn r#let(&mut self) -> Result<Let> {
        let span = self.curr.span;
        self.consume(TokenKind::Let)?;
        let ident = self.ident()?;

        let mut ty = None;
        if self.at(TokenKind::Colon) {
            self.consume(TokenKind::Colon)?;
            ty = Some(self.ty()?);
        }

        self.consume(TokenKind::Assign)?;
        let val = self.expr(Prec::Lowest)?;
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&self.curr.span);

        Ok(Let {
            ident,
            ty,
            val,
            id,
            span,
        })
    }

    fn r#if(&mut self) -> Result<IfElse> {
        let span = self.curr.span;
        self.consume(TokenKind::If)?;
        let condition = self.expr(Prec::Lowest)?;
        self.consume(TokenKind::Then)?;
        self.accept(TokenKind::Newline);

        let consequence = self.block()?;
        let alternative = if self.at(TokenKind::Else) {
            self.advance();
            self.skip_whitespace();
            Some(self.block()?)
        } else {
            None
        };
        let id = self.ctx.next_id();
        let tok = self.consume(TokenKind::End)?;
        let span = span.enclosing_to(&tok.span);

        Ok(IfElse {
            condition: Arc::from(condition),
            consequence,
            alternative,
            id,
            span,
        })
    }

    fn prefix_expr(&mut self, op: Op) -> Result<PrefixExpr> {
        let rhs = self.expr(Prec::default())?;
        let id = self.ctx.next_id();
        let span = op.span.enclosing_to(&rhs.span());

        Ok(PrefixExpr {
            op,
            rhs: Arc::from(rhs),
            id,
            span,
        })
    }

    fn op(&mut self) -> Result<(Op, Prec)> {
        let token = self.curr.to_owned();
        let id = self.ctx.next_id();
        let prec = Prec::from(&token);
        let span = token.span;

        let kind = match token.kind {
            TokenKind::Add => Ok(OpKind::Add),
            TokenKind::Sub => Ok(OpKind::Sub),
            TokenKind::Eq => Ok(OpKind::Eq),
            TokenKind::Asterisk => Ok(OpKind::Mul),
            TokenKind::Slash => Ok(OpKind::Div),
            TokenKind::LAngle => Ok(OpKind::Lt),
            TokenKind::RAngle => Ok(OpKind::Gt),
            TokenKind::And => Ok(OpKind::And),
            TokenKind::Or => Ok(OpKind::Or),
            _ => Err(ParseError::InvalidOperator {
                src: self.lex.source(),
                span: token.span(),
                help: None,
            }),
        }?;

        self.advance();

        let op = Op { id, span, kind };
        Ok((op, prec))
    }

    fn binop_expr(&mut self, lhs: Expr) -> Result<Expr> {
        let (op, prec) = self.op()?;
        let rhs = self.expr(prec)?;
        let id = self.ctx.next_id();
        let span = lhs.span().enclosing_to(&rhs.span());

        Ok(Expr::BinOp(BinOp {
            lhs: Arc::from(lhs),
            op,
            rhs: Arc::from(rhs),
            id,
            span,
        }))
    }

    fn call_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LParen)?;
        let params = if self.at(TokenKind::RParen) {
            vec![]
        } else {
            self.expr_list()?
        };
        let tok = self.consume(TokenKind::RParen)?;
        let id = self.ctx.next_id();
        let span = expr.span().enclosing_to(&tok.span());

        Ok(Expr::Call(CallExpr {
            func: Arc::from(expr),
            params: Arc::from(params),
            id,
            span,
        }))
    }

    fn index_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LBracket)?;
        let idx = self.expr(Prec::default())?;
        let tok = self.consume(TokenKind::RBracket)?;
        let id = self.ctx.next_id();
        let span = expr.span().enclosing_to(&tok.span());

        Ok(Expr::Index(IndexExpr {
            expr: expr.into(),
            idx: idx.into(),
            id,
            span,
        }))
    }

    fn member_access(&mut self, lhs: Expr) -> Result<Expr> {
        self.consume(TokenKind::Dot)?;
        let ident = self.ident()?;
        let lhs = Arc::from(lhs);
        let id = self.ctx.next_id();
        let span = lhs.span().enclosing_to(&ident.span);
        Ok(Expr::MemberAccess(MemberAccess {
            id,
            span,
            lhs,
            ident,
        }))
    }

    fn int_lit(&mut self) -> Result<Literal> {
        let text = &self.curr.text;
        let span = self.curr.span;
        let kind = text
            .parse()
            .map(LiteralKind::Int)
            .expect("unable to parse integer");
        self.advance();
        let id = self.ctx.next_id();
        Ok(Literal { id, span, kind })
    }

    fn str_lit(&mut self) -> Result<Literal> {
        let text = self.curr.text.clone();
        let span = self.curr.span;
        let kind = LiteralKind::Str(Arc::from(text));
        self.advance();
        let id = self.ctx.next_id();
        Ok(Literal { id, span, kind })
    }

    pub fn expr(&mut self, prec: Prec) -> Result<Expr> {
        let mut lhs = match self.curr.kind {
            TokenKind::Int => Expr::Literal(self.int_lit()?),
            TokenKind::String => Expr::Literal(self.str_lit()?),
            TokenKind::Ident => Expr::Ident(self.ident()?),
            TokenKind::If => Expr::IfElse(self.r#if()?),
            TokenKind::LBracket => Expr::List(self.list()?),
            tok if tok.is_prefix_operator() => {
                let (op, _prec) = self.op()?;
                let prefix_expr = self.prefix_expr(op)?;
                Expr::Prefix(prefix_expr)
            }

            _ => {
                return Err(ParseError::Todo {
                    src: self.lex.source(),
                    token: self.curr.clone(),
                    span: self.curr.span,
                });
            }
        };

        if self.at(TokenKind::Eof) {
            return Ok(lhs);
        }

        while prec < Prec::from(&self.curr) {
            if self.curr.kind.is_terminator() {
                break;
            }

            match self.curr.kind {
                kind if kind.is_operator() => {
                    lhs = self.binop_expr(lhs)?;
                }
                TokenKind::LParen => {
                    lhs = self.call_expr(lhs)?;
                }
                TokenKind::LSquirly => {
                    match lhs {
                        Expr::Ident(ident) => {
                            lhs = self.struct_constructor(ident)?;
                        }
                        _ => todo!(),
                    };
                }
                TokenKind::LBracket => {
                    lhs = self.index_expr(lhs)?;
                }
                TokenKind::Dot => {
                    lhs = self.member_access(lhs)?;
                }
                TokenKind::Newline => return Ok(lhs),
                _ => todo!("kind: {}, span: {:?}", self.curr.kind, self.curr.span),
            }
        }

        Ok(lhs)
    }

    fn expr_list(&mut self) -> Result<Vec<Expr>> {
        if self.at(TokenKind::RBracket) {
            return Ok(vec![]);
        }

        let head = self.expr(Prec::Lowest)?;
        let mut tail = vec![];
        tail.push(head);

        loop {
            if self.curr.kind != TokenKind::Comma {
                break;
            }

            self.consume(TokenKind::Comma)?;
            tail.push(self.expr(Prec::Lowest)?);
        }

        Ok(tail)
    }

    fn list(&mut self) -> Result<List> {
        let span = self.curr.span;
        self.consume(TokenKind::LBracket)?;
        let items = self.expr_list()?;
        let tok = self.consume(TokenKind::RBracket)?;

        let items = Arc::from(items);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&tok.span());
        Ok(List { items, id, span })
    }

    fn struct_def(&mut self) -> Result<StructDef> {
        let span = self.curr.span;
        self.consume(TokenKind::Struct)?;
        let ident = self.ident()?;
        self.consume(TokenKind::Assign)?;
        self.skip_whitespace();
        let fields = self.typed_params()?;
        let tok = self.consume(TokenKind::End)?;

        let fields = Arc::from(fields);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&tok.span());
        Ok(StructDef {
            ident,
            fields,
            id,
            span,
        })
    }

    fn struct_constructor(&mut self, ident: Ident) -> Result<Expr> {
        self.consume(TokenKind::LSquirly)?;
        let fields = self.value_params()?;
        let tok = self.consume(TokenKind::RSquirly)?;

        let fields = Arc::from(fields);
        let id = self.ctx.next_id();
        let span = ident.span.enclosing_to(&tok.span());
        Ok(Expr::Constructor(Constructor {
            ident,
            fields,
            id,
            span,
        }))
    }
}
