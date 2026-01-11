use miette::{Diagnostic, NamedSource, Result, SourceSpan, miette};
use thiserror::Error;

use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};
use crate::source::SourceInfo;

#[derive(Error, Diagnostic, Debug)]
pub enum ErrorKind {
    #[error("expected")]
    Expected {
        #[source_code]
        src: SourceInfo,

        #[label("this is of kind {actual} but was expected to be {expected}")]
        span: SourceSpan,

        expected: TokenKind,
        actual: TokenKind,

        #[help]
        help: Option<String>,
    },

    #[error("invalid type: {}", token.text)]
    InvalidType { token: Token },

    #[error("invalid operator: {}", token.text)]
    InvalidOperator { token: Token },

    #[error("unhandled token: {0:?}")]
    Todo(Token),
}

impl ErrorKind {
    fn into_error(self, parser: &Parser) -> miette::Report {
        let span = match self {
            ErrorKind::Todo(ref token) => token.span,
            ErrorKind::InvalidType { ref token } => token.span,
            _ => (parser.lex.pos, 1).into(),
        };

        ParseError {
            kind: self,
            span,
            src: NamedSource::new("mysource", parser.lex.content.clone()),
        }
        .into()
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("{kind:#}")]
#[diagnostic(code(parser))]
pub struct ParseError {
    #[source_code]
    src: NamedSource<String>,

    #[label("This bit here 💩")]
    span: SourceSpan,

    #[diagnostic(transparent)]
    kind: ErrorKind,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum Prec {
    #[default]
    Lowest,
    AndOr,   // && or || - lower precedence than equality
    Eq,      // ==
    Cmp,     // > or <
    Sum,     // +
    Product, // *
    Prefix,  // -a, !a or &a
    Call,    // func()
    // Index, // list[0]
    Chain, // mod.field
}

impl From<&Token> for Prec {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add | TokenKind::Sub => Self::Sum,
            TokenKind::Eq => Self::Eq,
            TokenKind::LParen => Self::Call,
            TokenKind::LAngle | TokenKind::RAngle => Self::Cmp,
            TokenKind::Asterisk => Self::Product,
            TokenKind::And | TokenKind::Or => Self::AndOr,
            TokenKind::Dot => Self::Chain,
            TokenKind::Bang | TokenKind::Ampersand => Self::Prefix,
            _ => Self::Lowest,
        }
    }
}

impl Op {
    fn try_from_token(token: Token, id: NodeId) -> Result<Op> {
        let span = token.span;
        let kind = match token.kind {
            TokenKind::Add => OpKind::Add,
            TokenKind::Sub => OpKind::Sub,
            TokenKind::Eq => OpKind::Eq,
            TokenKind::Asterisk => OpKind::Mul,
            TokenKind::Slash => OpKind::Div,
            TokenKind::LAngle => OpKind::Lt,
            TokenKind::RAngle => OpKind::Gt,
            TokenKind::And => OpKind::And,
            TokenKind::Or => OpKind::Or,
            TokenKind::Dot => OpKind::Chain,
            _ => return Err(ErrorKind::InvalidOperator { token }.into()),
        };
        Ok(Op { id, span, kind })
    }
}

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

/// Take beginning of the first span and the end of the second span
/// and return a new span covering this whole range
fn enclosing_span(a: Span, b: Span) -> Span {
    let offset = a.offset();
    let len = b.offset() - a.offset() + b.len();
    Span::from((offset, len))
}

pub struct Parser {
    pub lex: Lexer,
    pub ctx: Context,
    pub tokens: Vec<Token>,
    pub curr: Token,
    pub next: Option<Token>,
}

impl Parser {
    pub fn new(content: impl ToString) -> Self {
        let mut lex = Lexer::new(content);
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
            if self.curr.kind == TokenKind::Eof {
                break;
            }

            self.skip_whitespace();
            match self.node() {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    err.downcast()?;
                }
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
            return Err(ErrorKind::Expected {
                src: todo!(),
                span: todo!(),
                expected,
                actual: todo!(),
                help: todo!(),
            }
            .into_error(self));
        }
        Ok(self.curr.clone())
    }

    fn accept(&mut self, expected: TokenKind) -> Option<Token> {
        if self.curr.kind == expected {
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
        while self.curr.kind == TokenKind::Newline {
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

        let span = enclosing_span(span, self.curr.span);
        let id = self.ctx.next_id();
        Ok(Block { nodes, id, span })
    }

    fn ident(&mut self) -> Result<Ident> {
        let token = self.consume(TokenKind::Ident)?;
        let id = self.ctx.next_id();
        Ok(Ident {
            id,
            span: token.span,
            inner: token.text,
        })
    }

    fn ty(&mut self) -> Result<Ty> {
        let span = self.curr.span;
        let ident = self.ident()?;
        let kind = match ident.as_ref() {
            "i8" => TyKind::Int(IntTyKind::I8),
            "i16" => TyKind::Int(IntTyKind::I16),
            "i32" => TyKind::Int(IntTyKind::I32),
            "i64" => TyKind::Int(IntTyKind::I64),
            "u8" => TyKind::Int(IntTyKind::U8),
            "u16" => TyKind::Int(IntTyKind::U16),
            "u32" => TyKind::Int(IntTyKind::U32),
            "u64" => TyKind::Int(IntTyKind::U64),
            "Bool" => TyKind::Bool,
            "Str" => TyKind::Str,
            _ => TyKind::Var(ident),
        };
        let id = self.ctx.next_id();
        let span = enclosing_span(span, self.curr.span);
        let mut ty = Ty { kind, id, span };

        if self.at(TokenKind::LBracket) {
            self.consume(TokenKind::LBracket)?;
            self.consume(TokenKind::RBracket)?;
            let kind = TyKind::List {
                inner: Box::new(ty),
                size: None,
            };
            let id = self.ctx.next_id();
            let span = enclosing_span(span, self.curr.span);
            ty = Ty { kind, id, span }
        }

        Ok(ty)
    }

    fn func(&mut self) -> Result<Fn> {
        let span = self.curr.span;
        let is_extern = self.curr.kind == TokenKind::Extern;
        if is_extern {
            self.advance();
        }

        self.consume(TokenKind::Fn).unwrap();

        let ident = self
            .ident()
            .map_err(|_| miette!("expected ident, got: {:?}", self.curr))?;

        self.consume(TokenKind::LParen)?;
        let mut params = vec![];
        while self.curr.kind != TokenKind::RParen {
            params.push(self.typed_param()?);
            if self.curr.kind == TokenKind::Comma {
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

            let id = self.ctx.next_id();
            let span = enclosing_span(span, self.curr.span);
            Some(Block { nodes, id, span })
        };

        let id = self.ctx.next_id();
        let span = enclosing_span(span, self.curr.span);
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
        let span = enclosing_span(span, ident.span);
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
        if self.curr.kind == TokenKind::Comma {
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
                let span = enclosing_span(span, self.curr.span);
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
        if self.curr.kind == TokenKind::Colon {
            self.consume(TokenKind::Colon)?;
            ty = Some(self.ty()?);
        }

        self.consume(TokenKind::Assign)?;
        let val = self.expr(Prec::Lowest)?;
        let id = self.ctx.next_id();
        let span = enclosing_span(span, self.curr.span);

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
        let alternative = if self.curr.kind == TokenKind::Else {
            self.advance();
            self.skip_whitespace();
            Some(self.block()?)
        } else {
            None
        };
        let id = self.ctx.next_id();
        let tok = self.consume(TokenKind::End)?;
        let span = enclosing_span(span, tok.span());

        Ok(IfElse {
            condition: Box::new(condition),
            consequence,
            alternative,
            id,
            span,
        })
    }

    fn prefix_expr(&mut self, op: Op) -> Result<Expr> {
        let rhs = self.expr(Prec::default())?;
        let id = self.ctx.next_id();
        let span = enclosing_span(op.span, rhs.span());

        Ok(Expr::Prefix(PrefixExpr {
            op,
            rhs: Box::new(rhs),
            id,
            span,
        }))
    }

    fn binop_expr(&mut self, lhs: Expr) -> Result<Expr> {
        if !self.curr.kind.is_operator() {
            panic!("invalid operator");
        }
        let token = self.curr.to_owned();
        let id = self.ctx.next_id();
        let op = Op::try_from_token(token, id)?;
        let prec = Prec::from(&self.curr);
        self.advance();

        let rhs = self.expr(prec)?;
        let id = self.ctx.next_id();
        let span = enclosing_span(lhs.span(), self.curr.span);

        Ok(Expr::BinOp(BinOp {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
            id,
            span,
        }))
    }

    fn call_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LParen)?;
        let args = if self.at(TokenKind::RParen) {
            vec![]
        } else {
            self.expr_list()?
        };
        let tok = self.consume(TokenKind::RParen)?;
        let id = self.ctx.next_id();
        let span = enclosing_span(expr.span(), tok.span());

        Ok(Expr::Call(CallExpr {
            func: Box::new(expr),
            params: args,
            id,
            span,
        }))
    }

    fn index_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LBracket)?;
        let idx = self.expr(Prec::default())?;
        let tok = self.consume(TokenKind::RBracket)?;
        let id = self.ctx.next_id();
        let span = enclosing_span(expr.span(), tok.span());

        Ok(Expr::Index(IndexExpr {
            expr: expr.into(),
            idx: idx.into(),
            id,
            span,
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
        let kind = LiteralKind::Str(text);
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
            _ => panic!("{:?}", ErrorKind::Todo(self.curr.clone()).into_error(self)),
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
                TokenKind::Newline => return Ok(lhs),
                _ => todo!("kind: {}, span: {:?}", self.curr.kind, self.curr.span),
            }
        }

        Ok(lhs)
    }

    fn expr_list(&mut self) -> Result<Vec<Expr>> {
        if self.curr.kind == TokenKind::RBracket {
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
        let id = self.ctx.next_id();
        let span = enclosing_span(span, tok.span());
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
        let id = self.ctx.next_id();
        let span = enclosing_span(span, tok.span());
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
        let id = self.ctx.next_id();
        let span = enclosing_span(ident.span, tok.span());
        Ok(Expr::Constructor(Constructor {
            ident,
            fields,
            id,
            span,
        }))
    }
}
