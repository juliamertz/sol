use std::path::PathBuf;
use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::*;
use crate::ext::{AsStr, Boxed};
use crate::interner::Id;
use crate::lexer::source::{SourceInfo, Span};
use crate::lexer::token::OwnedToken;
use crate::lexer::{Lexer, Token, TokenKind};

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

    #[error(transparent)]
    Lexer(#[from] crate::lexer::LexerError),
}

pub type Result<T, E = ParseError> = core::result::Result<T, E>;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum Prec {
    #[default]
    Lowest,
    Assign,    // a = 10
    AndOr,     // && or || - lower precedence than equality
    Eq,        // ==
    Cmp,       // > or <
    Sum,       // +
    Product,   // *
    Unary,     // -a, !a or &a
    Call,      // func()
    Construct, // Point { x : 10, y : 5 }
    Index,     // list[0]
    Chain,     // mod.field
}

impl From<&Token<'_>> for Prec {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add | TokenKind::Sub => Self::Sum,
            TokenKind::Assign => Self::Assign,
            TokenKind::Eq => Self::Eq,
            TokenKind::LParen => Self::Call,
            TokenKind::LSquirly => Self::Construct,
            TokenKind::LBracket => Self::Index,
            TokenKind::LAngle | TokenKind::RAngle => Self::Cmp,
            TokenKind::Asterisk => Self::Product,
            TokenKind::And | TokenKind::Or => Self::AndOr,
            TokenKind::Dot => Self::Chain,
            TokenKind::Bang | TokenKind::Ampersand => Self::Unary,
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

pub struct Parser<'src> {
    pub lex: Lexer<'src>,
    pub ctx: Context,
    pub tokens: Vec<Token<'src>>,
    pub curr: Token<'src>,
    pub next: Option<Token<'src>>,
}

impl<'src> Parser<'src> {
    pub fn new(file_path: PathBuf, content: &'src str) -> Result<Self> {
        let mut lex = Lexer::new(file_path, content);
        let curr = lex
            .read_token()
            .transpose()?
            .unwrap_or(Token::new(TokenKind::Eof, "", lex.pos));
        let next = lex.read_token().transpose()?;
        let ctx = Context::default();
        Ok(Self {
            lex,
            ctx,
            curr,
            next,
            tokens: vec![],
        })
    }

    pub fn parse(&mut self) -> Result<Module> {
        let mut items = vec![];

        loop {
            if self.at(TokenKind::Eof) {
                break;
            }

            self.skip_whitespace()?;
            match self.item() {
                Ok(item) => items.push(item),
                Err(err) => return Err(err),
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

        self.next = self.lex.read_token().transpose()?;
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

    fn skip_whitespace(&mut self) -> Result<()> {
        while self.at(TokenKind::Newline) {
            self.advance()?;
        }
        Ok(())
    }

    pub fn item(&mut self) -> Result<Item> {
        let item = match self.curr.kind {
            TokenKind::Fn => Item::Fn(self.func()?),
            TokenKind::Extern => Item::Fn(self.extern_func()?),
            TokenKind::Use => Item::Use(self.r#use()?),
            TokenKind::Struct => Item::StructDef(self.struct_def()?),
            TokenKind::Impl => Item::Impl(self.imp()?),
            _ => {
                return Err(ParseError::Todo {
                    src: self.lex.source(),
                    token: self.curr.owned(),
                    span: self.curr.span,
                });
            }
        };

        self.skip_whitespace()?;

        Ok(item)
    }

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

    fn name(&mut self) -> Result<Name> {
        let token = self.consume(TokenKind::Ident)?;
        Ok(Name {
            span: token.span,
            inner: Arc::from(token.text),
        })
    }

    fn ty(&mut self) -> Result<Ty> {
        let span = self.curr.span;
        let ident = self.ident()?;
        let kind = match ident.as_str() {
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

        if self.accept(TokenKind::LBracket)?.is_some() {
            let size = if self.at(TokenKind::Int) {
                let lit = self.int_lit()?;
                // this is janky 😭
                let LiteralKind::Int(size) = lit.kind else {
                    unreachable!();
                };
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

    fn r#use(&mut self) -> Result<Use> {
        let span = self.curr.span;
        self.consume(TokenKind::Use)?;
        let is_extern = self.accept(TokenKind::Extern)?.is_some();
        let name = self.name()?;
        let span = span.enclosing_to(&self.curr.span);
        Ok(Use {
            span,
            is_extern,
            name,
        })
    }

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

    fn unary_op(&mut self) -> Result<(Op<UnaryOpKind>, Prec)> {
        let token = self.curr.to_owned();
        let prec = Prec::from(&token);
        let span = token.span;

        let kind = match token.kind {
            TokenKind::Sub => Ok(UnaryOpKind::Negate),
            TokenKind::Bang => Ok(UnaryOpKind::Not),
            _ => Err(ParseError::InvalidOperator {
                src: self.lex.source(),
                span: token.span(),
                help: None,
            }),
        }?;

        self.advance()?;

        let op = Op { span, kind };
        Ok((op, prec))
    }

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

    fn bin_op(&mut self) -> Result<(Op<BinOpKind>, Prec)> {
        let token = self.curr.to_owned();
        let prec = Prec::from(&token);
        let span = token.span;

        let kind = match token.kind {
            TokenKind::Add => Ok(BinOpKind::Add),
            TokenKind::Sub => Ok(BinOpKind::Sub),
            TokenKind::Eq => Ok(BinOpKind::Eq),
            TokenKind::Asterisk => Ok(BinOpKind::Mul),
            TokenKind::Slash => Ok(BinOpKind::Div),
            TokenKind::LAngle => Ok(BinOpKind::Lt),
            TokenKind::RAngle => Ok(BinOpKind::Gt),
            TokenKind::And => Ok(BinOpKind::And),
            TokenKind::Or => Ok(BinOpKind::Or),
            _ => Err(ParseError::InvalidOperator {
                src: self.lex.source(),
                span: token.span(),
                help: None,
            }),
        }?;

        self.advance()?;

        let op = Op { span, kind };
        Ok((op, prec))
    }

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

    fn member_access(&mut self, lhs: Expr) -> Result<Expr> {
        self.consume(TokenKind::Dot)?;
        let rhs = self.name()?;
        let lhs = Arc::from(lhs);
        let id = self.ctx.next_id();
        let span = lhs.span().enclosing_to(&rhs.span);
        Ok(Expr::MemberAccess(MemberAccess { id, span, lhs, rhs }))
    }

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

    fn int_lit(&mut self) -> Result<Literal> {
        let text = &self.curr.text;
        let span = self.curr.span;
        let kind = text
            .parse()
            .map(LiteralKind::Int)
            .expect("unable to parse integer");
        self.advance()?;
        let id = self.ctx.next_id();
        Ok(Literal { id, span, kind })
    }

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
        Ok(Literal { id, span, kind })
    }

    fn str_lit(&mut self) -> Result<Literal> {
        let text = self.curr.text;
        let span = self.curr.span;
        let kind = LiteralKind::Str(Arc::from(text));
        self.advance()?;
        let id = self.ctx.next_id();
        Ok(Literal { id, span, kind })
    }

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

    pub fn expr(&mut self, prec: Prec) -> Result<Expr> {
        let mut lhs = match self.curr.kind {
            TokenKind::Int => Expr::Literal(self.int_lit()?),
            TokenKind::True | TokenKind::False => Expr::Literal(self.bool_lit()?),
            TokenKind::String => Expr::Literal(self.str_lit()?),
            TokenKind::Ident => Expr::Ident(self.ident()?),
            TokenKind::If => Expr::IfElse(self.r#if()?),
            TokenKind::LBracket => Expr::List(self.list()?),
            TokenKind::While => Expr::While(self.while_loop()?),

            tok if tok.is_unary_op() => {
                let (op, _prec) = self.unary_op()?;
                let unary = self.unary(op)?;
                Expr::Unary(unary)
            }

            _ => {
                return Err(ParseError::Todo {
                    src: self.lex.source(),
                    token: self.curr.owned(),
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

    pub fn expr_lowest(&mut self) -> Result<Expr> {
        self.expr(Prec::Lowest)
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
        let span = self.curr.span();
        self.consume(TokenKind::LBracket)?;
        let items = self.expr_list()?;
        let tok = self.consume(TokenKind::RBracket)?;

        let items = Arc::from(items);
        let id = self.ctx.next_id();
        let span = span.enclosing_to(&tok.span());
        Ok(List { items, id, span })
    }

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
