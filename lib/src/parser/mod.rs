use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;
use tracing::instrument;

use crate::ast::*;
use crate::interner::Id;
use crate::lexer::source::{SourceInfo, Span};
use crate::lexer::token::OwnedToken;
use crate::lexer::{Lexer, Token, TokenKind};

use prec::Prec;

mod num;
mod prec;
mod resolve;
#[cfg(test)]
mod test;

#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(solc::parser))]
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

        token: OwnedToken,

        #[label("this token")]
        span: Span,
    },

    #[error("array size must be a constant integer literal")]
    NonLiteralArraySize {
        #[source_code]
        src: SourceInfo,

        #[label("here")]
        span: Span,
    },

    #[error(transparent)]
    Lexer(#[from] crate::lexer::LexerError),
    #[error(transparent)]
    ParseNumber(#[from] num::ParseNumberError),
}

type Result<T, E = ParseError> = core::result::Result<T, E>;

#[derive(Default)]
struct Context {
    id: u32,
}

impl Context {
    fn next_id(&mut self) -> NodeId {
        let id = self.id;
        self.id += 1;
        NodeId::new(id)
    }
}

pub struct Parser<'src> {
    lex: Lexer<'src>,
    ctx: Context,
    tokens: Vec<Token<'src>>,
    curr: Token<'src>,
    next: Option<Token<'src>>,
}

impl<'src> Parser<'src> {
    pub fn new(file_path: impl AsRef<std::path::Path>, content: &'src str) -> Result<Self> {
        let mut lex = Lexer::new(file_path, content);
        let curr = lex
            .next()
            .transpose()?
            .unwrap_or(Token::new(TokenKind::Eof, "", lex.pos()));
        let next = lex.next().transpose()?;
        let ctx = Context::default();
        Ok(Self {
            lex,
            ctx,
            curr,
            next,
            tokens: vec![],
        })
    }

    pub fn source(&self) -> SourceInfo {
        self.lex.source()
    }

    #[instrument(skip_all, err(Debug))]
    pub fn module(&mut self) -> Result<Module> {
        let mut items = vec![];

        loop {
            if self.at(TokenKind::Eof) {
                break;
            }

            self.skip_whitespace()?;
            {
                let item = self.item()?;
                items.push(item)
            }
        }

        Ok(Module {
            items: Arc::from(items),
        })
    }

    fn advance(&mut self) -> Result<Option<Token<'src>>> {
        let curr = self.next.clone();
        if let Some(next) = self.next.clone() {
            self.curr = next;
        }

        self.next = self.lex.next().transpose()?;
        Ok(if let Some(next) = &self.next {
            self.tokens.push(next.clone());
            curr
        } else {
            None
        })
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token<'src>> {
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

    fn accept(&mut self, expected: TokenKind) -> Result<Option<Token<'src>>> {
        if self.at(expected) {
            let tok = self.curr.clone();
            self.advance()?;
            Ok(Some(tok))
        } else {
            Ok(None)
        }
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token<'src>> {
        let tok = self.expect(expected)?;
        self.advance()?;
        Ok(tok)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.curr.kind == kind
    }

    #[instrument(skip_all, err(Debug))]
    fn skip_whitespace(&mut self) -> Result<()> {
        while self.at(TokenKind::Newline) {
            self.advance()?;
        }
        Ok(())
    }

    #[instrument(skip_all, err(Debug))]
    fn item(&mut self) -> Result<Item> {
        let item = match self.curr.kind {
            TokenKind::Fn => Item::Fn(self.func()?),
            TokenKind::Use => Item::Use(self.r#use()?),
            TokenKind::Extern => match self.next.as_ref().map(|tok| tok.kind) {
                Some(TokenKind::Fn | TokenKind::Variadic) => Item::Fn(self.extern_func()?),
                Some(TokenKind::Use) => Item::Use(self.r#use()?),
                _ => todo!("error: extern keyword should be followed by func or use"),
            },
            TokenKind::Struct => Item::StructDef(self.struct_def()?),
            TokenKind::Impl => Item::Impl(self.imp()?),
            _ => {
                return Err(ParseError::Todo {
                    src: self.lex.source(),
                    token: self.curr.to_owned(),
                    span: self.curr.span,
                });
            }
        };

        self.skip_whitespace()?;

        Ok(item)
    }

    #[instrument(skip_all, err(Debug))]
    fn block(&mut self) -> Result<Block> {
        let span = self.curr.span;
        let mut nodes = vec![];
        loop {
            if self.curr.kind.is_terminator() {
                break;
            }
            nodes.push(self.stmnt()?);
        }

        let nodes = Arc::from(nodes);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&self.curr.span);
        Ok(Block { nodes, id, span })
    }

    #[instrument(skip_all, err(Debug))]
    fn ident(&mut self) -> Result<Ident> {
        let token = self.consume(TokenKind::Ident)?;
        let id = self.ctx.next_id();
        Ok(Ident {
            id,
            span: token.span,
            inner: Arc::from(token.text),
            is_extern: false,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn name(&mut self) -> Result<Name> {
        let token = self.consume(TokenKind::Ident)?;
        Ok(Name {
            span: token.span,
            inner: Arc::from(token.text),
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn ty(&mut self) -> Result<Ty> {
        let span = self.curr.span;
        let id = self.ctx.next_id();
        let is_ptr = self.accept(TokenKind::Asterisk)?.is_some();
        let ident = self.ident()?;
        let bare_kind = TyKind::from_ident(ident);
        let kind = if is_ptr {
            TyKind::Ptr(Arc::new(Ty {
                id: self.ctx.next_id(),
                span: self.curr.span, // TODO: not correct
                kind: bare_kind,
            }))
        } else {
            bare_kind
        };
        let span = span.enclosing_to(&self.curr.span);
        let mut ty = Ty { kind, id, span };

        if self.accept(TokenKind::LBracket)?.is_some() {
            let size = if let TokenKind::Num(num) = self.curr.kind {
                let size = self
                    .num_lit(num)?
                    .as_int()
                    .ok_or(ParseError::NonLiteralArraySize {
                        src: self.source(),
                        span,
                    })?;
                Some(size as usize)
            } else {
                None
            };

            self.consume(TokenKind::RBracket)?;
            let kind = TyKind::List {
                inner: Arc::from(ty),
                size,
            };
            let id = self.ctx.next_id();
            let span = span.enclosing_to(&self.curr.span);
            ty = Ty { kind, id, span }
        }

        Ok(ty)
    }

    #[instrument(skip_all, err(Debug))]
    fn func(&mut self) -> Result<Fn> {
        let span = self.curr.span;
        self.consume(TokenKind::Fn)?;
        let ident = self.ident()?;
        self.consume(TokenKind::LParen)?;
        let params = self.params(Self::ident, Self::ty)?;
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::Arrow)?;
        let return_ty = self.ty()?;
        self.skip_whitespace()?;

        let body = {
            let span = self.curr.span;
            let mut nodes = vec![];
            while self.curr.kind != TokenKind::End {
                nodes.push(self.stmnt()?);
            }

            self.consume(TokenKind::End)?;

            let nodes = Arc::from(nodes);
            let id = self.ctx.next_id();
            let span = span.enclosing_to(&self.curr.span);
            Block { nodes, id, span }
        };

        let params = Arc::from(params);
        let span = span.enclosing_to(&self.curr.span);
        Ok(Fn {
            span,
            ident,
            kind: FnKind::Local { params, body },
            return_ty,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn extern_func(&mut self) -> Result<Fn> {
        let span = self.curr.span;
        self.consume(TokenKind::Extern)?;
        let is_variadic = self.accept(TokenKind::Variadic)?.is_some();
        self.consume(TokenKind::Fn)?;
        let ident = self.ident()?;
        self.consume(TokenKind::LParen)?;
        let params = self.params(Self::name, Self::ty)?;
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::Arrow)?;
        let return_ty = self.ty()?;
        self.skip_whitespace()?;

        let params = Arc::from(params);
        let span = span.enclosing_to(&self.curr.span);
        Ok(Fn {
            span,
            ident,
            kind: FnKind::Extern {
                params,
                is_variadic,
            },
            return_ty,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn path(&mut self) -> Result<Path> {
        let mut segments = vec![];

        loop {
            match self.curr.kind {
                TokenKind::Ident => {
                    segments.push(PathSegment::Name(self.name()?));
                }
                TokenKind::Slash => continue,
                TokenKind::Newline => break,
                _ => todo!("illegal token in path"),
            }
        }

        Ok(Path::from_segments(segments))
    }

    #[instrument(skip_all, err(Debug))]
    fn r#use(&mut self) -> Result<Use> {
        let span = self.curr.span;
        let is_extern = self.accept(TokenKind::Extern)?.is_some();
        self.consume(TokenKind::Use)?;
        let path = self.path()?;
        let span = span.enclosing_to(&self.curr.span);
        Ok(Use {
            span,
            is_extern,
            path,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn params<K, V>(
        &mut self,
        parse_key: fn(&mut Parser<'src>) -> Result<K>,
        parse_val: fn(&mut Parser<'src>) -> Result<V>,
    ) -> Result<Vec<(K, V)>> {
        let mut args = vec![];

        loop {
            self.skip_whitespace()?;

            if self.curr.kind.is_terminator() {
                break;
            }

            args.push({
                let key = parse_key(self)?;
                self.consume(TokenKind::Colon)?;
                let val = parse_val(self)?;
                (key, val)
            });

            if self.at(TokenKind::Comma) {
                self.advance()?;
            }
        }

        Ok(args)
    }

    #[instrument(skip_all, err(Debug))]
    fn stmnt(&mut self) -> Result<Stmnt> {
        let stmnt = match self.curr.kind {
            TokenKind::Let => Stmnt::Let(self.r#let()?),
            TokenKind::Ret => {
                let span = self.curr.span;
                self.consume(TokenKind::Ret)?;
                let val = self.expr(Prec::default())?;
                let span = span.enclosing_to(&self.curr.span);
                Stmnt::Ret(Ret { val, span })
            }
            _ => Stmnt::Expr(self.expr(Prec::default())?),
        };

        self.skip_whitespace()?;

        Ok(stmnt)
    }

    #[instrument(skip_all, err(Debug))]
    fn r#let(&mut self) -> Result<Let> {
        let span = self.curr.span;
        self.consume(TokenKind::Let)?;
        let mutable = self.accept(TokenKind::Mut)?.is_some();
        let ident = self.ident()?;

        let ty = self
            .accept(TokenKind::Colon)?
            .map(|_| self.ty())
            .transpose()?;

        self.consume(TokenKind::Assign)?;
        let val = self.expr(Prec::Lowest)?;
        let span = span.enclosing_to(&val.span());

        Ok(Let {
            span,
            mutable,
            ident,
            ty,
            val,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn r#if(&mut self) -> Result<IfElse> {
        let span = self.curr.span;
        self.consume(TokenKind::If)?;
        let condition = self.expr(Prec::Lowest)?;
        self.consume(TokenKind::Then)?;
        self.accept(TokenKind::Newline)?;

        let consequence = self.block()?;
        let alternative = if self.at(TokenKind::Else) {
            self.advance()?;
            self.skip_whitespace()?;
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

    #[instrument(skip_all, err(Debug))]
    fn unary_op(&mut self) -> Result<(Op<UnaryOpKind>, Prec)> {
        let prec = Prec::from(&self.curr);
        let span = self.curr.span;
        let kind = match self.curr.kind {
            TokenKind::Sub => Ok(UnaryOpKind::Negate),
            TokenKind::Bang => Ok(UnaryOpKind::Not),
            _ => Err(ParseError::InvalidOperator {
                src: self.lex.source(),
                span,
                help: None,
            }),
        }?;

        self.advance()?;

        Ok((Op { span, kind }, prec))
    }

    #[instrument(skip_all, err(Debug))]
    fn unary(&mut self, op: Op<UnaryOpKind>) -> Result<Unary> {
        let rhs = self.expr(Prec::default())?;
        let id = self.ctx.next_id();
        let span = op.span.enclosing_to(&rhs.span());

        Ok(Unary {
            op,
            rhs: Arc::from(rhs),
            id,
            span,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn bin_op(&mut self) -> Result<(Op<BinOpKind>, Prec)> {
        let prec = Prec::from(&self.curr);
        let span = self.curr.span;
        let kind = match self.curr.kind {
            TokenKind::Add => Ok(BinOpKind::Add),
            TokenKind::Sub => Ok(BinOpKind::Sub),
            TokenKind::Eq => Ok(BinOpKind::Eq),
            TokenKind::Ne => Ok(BinOpKind::Ne),
            TokenKind::Asterisk => Ok(BinOpKind::Mul),
            TokenKind::Slash => Ok(BinOpKind::Div),
            TokenKind::LAngle => Ok(BinOpKind::Lt),
            TokenKind::RAngle => Ok(BinOpKind::Gt),
            TokenKind::And => Ok(BinOpKind::And),
            TokenKind::Or => Ok(BinOpKind::Or),
            _ => Err(ParseError::InvalidOperator {
                src: self.lex.source(),
                span,
                help: None,
            }),
        }?;

        self.advance()?;

        Ok((Op { span, kind }, prec))
    }

    #[instrument(skip_all, err(Debug))]
    fn binop_expr(&mut self, lhs: Expr) -> Result<Expr> {
        let (op, prec) = self.bin_op()?;
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

    #[instrument(skip_all, err(Debug))]
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

        Ok(Expr::Call(Call {
            func: Arc::from(expr),
            params: Arc::from(params),
            id,
            span,
        }))
    }

    #[instrument(skip_all, err(Debug))]
    fn index_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LBracket)?;
        let idx = self.expr(Prec::default())?;
        let tok = self.consume(TokenKind::RBracket)?;
        let id = self.ctx.next_id();
        let span = expr.span().enclosing_to(&tok.span());

        Ok(Expr::Index(Index {
            expr: expr.into(),
            idx: idx.into(),
            id,
            span,
        }))
    }

    #[instrument(skip_all, err(Debug))]
    fn member_access(&mut self, lhs: Expr) -> Result<Expr> {
        self.consume(TokenKind::Dot)?;
        let rhs = self.name()?;
        let lhs = Arc::from(lhs);
        let id = self.ctx.next_id();
        let span = lhs.span().enclosing_to(&rhs.span);
        Ok(Expr::MemberAccess(MemberAccess { id, span, lhs, rhs }))
    }

    #[instrument(skip_all, err(Debug))]
    fn assign(&mut self, lhs: Expr) -> Result<Expr> {
        self.consume(TokenKind::Assign)?;
        let id = self.ctx.next_id();
        let rhs = self.expr(Prec::default())?;
        let span = lhs.span().enclosing_to(&rhs.span());

        Ok(Expr::Assign(Assign {
            id,
            span,
            lhs: Arc::from(lhs),
            rhs: Arc::from(rhs),
        }))
    }

    #[instrument(skip_all, err(Debug))]
    fn bool_lit(&mut self) -> Result<Literal> {
        let span = self.curr.span;
        let val = if self.at(TokenKind::True) {
            true
        } else if self.at(TokenKind::False) {
            false
        } else {
            unreachable!()
        };
        let kind = LiteralKind::Bool(val);
        self.advance()?;
        let id = self.ctx.next_id();
        Ok(Literal {
            id,
            span,
            kind,
            suffix: None,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn str_lit(&mut self) -> Result<Literal> {
        let text = &self.curr.text;
        let span = self.curr.span;
        let kind = LiteralKind::Str(text.to_string());
        self.advance()?;
        let id = self.ctx.next_id();
        Ok(Literal {
            id,
            span,
            kind,
            suffix: None,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn while_loop(&mut self) -> Result<While> {
        let span = self.curr.span();
        self.consume(TokenKind::While)?;
        let condition = self.expr(Prec::default())?.into();
        self.consume(TokenKind::Do)?;
        self.skip_whitespace()?;
        let consequence = self.block()?;
        let end = self.consume(TokenKind::End)?;
        let span = span.enclosing_to(&end.span);
        let id = self.ctx.next_id();
        Ok(While {
            id,
            span,
            label: None,
            condition,
            consequence,
        })
    }

    #[instrument(skip_all, err(Debug))]
    pub(crate) fn expr(&mut self, prec: Prec) -> Result<Expr> {
        let mut lhs = match self.curr.kind {
            TokenKind::Num(num) => Expr::Literal(self.num_lit(num)?),
            TokenKind::True | TokenKind::False => Expr::Literal(self.bool_lit()?),
            TokenKind::String => Expr::Literal(self.str_lit()?),
            TokenKind::Ident => Expr::Ident(self.ident()?),
            TokenKind::If => Expr::IfElse(self.r#if()?),
            TokenKind::LBracket => Expr::List(self.list()?),
            TokenKind::Asterisk => {
                self.advance()?;
                Expr::Deref(self.expr(Prec::Unary)?.into())
            }
            TokenKind::While => Expr::While(self.while_loop()?),

            tok if tok.is_unary_op() => {
                let (op, _prec) = self.unary_op()?;
                let unary = self.unary(op)?;
                Expr::Unary(unary)
            }

            _ => {
                return Err(ParseError::Todo {
                    src: self.lex.source(),
                    token: self.curr.to_owned(),
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

                TokenKind::Assign => {
                    lhs = self.assign(lhs)?;
                }

                _ => todo!("kind: {}, span: {:?}", self.curr.kind, self.curr.span),
            }
        }

        Ok(lhs)
    }

    #[instrument(skip_all, err(Debug))]
    fn expr_lowest(&mut self) -> Result<Expr> {
        self.expr(Prec::Lowest)
    }

    #[instrument(skip_all, err(Debug))]
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

    #[instrument(skip_all, err(Debug))]
    fn list(&mut self) -> Result<List> {
        let span = self.curr.span();
        self.consume(TokenKind::LBracket)?;
        let items = self.expr_list()?;
        let tok = self.consume(TokenKind::RBracket)?;

        let items = Arc::from(items);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&tok.span());
        Ok(List { items, id, span })
    }

    #[instrument(skip_all, err(Debug))]
    fn struct_def(&mut self) -> Result<StructDef> {
        let span = self.curr.span();
        self.consume(TokenKind::Struct)?;
        let ident = self.ident()?;
        self.consume(TokenKind::Assign)?;
        self.skip_whitespace()?;
        let fields = self.params(Self::name, Self::ty)?;
        let tok = self.consume(TokenKind::End)?;

        let fields = Arc::from(fields);
        let span = span.enclosing_to(&tok.span());
        Ok(StructDef {
            ident,
            fields,
            span,
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn imp(&mut self) -> Result<Impl> {
        let span = self.curr.span();
        self.consume(TokenKind::Impl)?;
        let ident = self.ident()?;
        self.consume(TokenKind::Assign)?;
        self.skip_whitespace()?;

        let mut items = vec![];
        loop {
            self.skip_whitespace()?;
            if self.at(TokenKind::End) {
                break;
            }

            items.push(AssocItem::Fn(self.func()?));
        }

        let tok = self.consume(TokenKind::End)?;
        let span = span.enclosing_to(&tok.span());
        Ok(Impl {
            span,
            ident,
            items: Arc::from(items),
        })
    }

    #[instrument(skip_all, err(Debug))]
    fn struct_constructor(&mut self, ident: Ident) -> Result<Expr> {
        self.consume(TokenKind::LSquirly)?;
        let fields = self.params(Self::name, Self::expr_lowest)?;
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
