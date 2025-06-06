use crate::ast::{Block, CallExpr, Expr, Fn, FnArg, Ident, InfixExpr, Node, Op, Stmnt, Type, Use};
use crate::lexer::{Lexer, Token, TokenKind};

use miette::{
    Context, Diagnostic, IntoDiagnostic, NamedSource, Result, SourceOffset, SourceSpan, miette,
};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum ErrorKind {
    #[error("Illegal token")]
    #[diagnostic(code(my_lib::bad_code))]
    Illegal,

    #[error("Unexpected EOF")]
    #[diagnostic(code(my_lib::bad_code))]
    UnexpectedEOF,

    #[error("Invalid operator")]
    #[diagnostic(code(my_lib::bad_code))]
    InvalidOperator,

    #[error("expected token {0}")]
    #[diagnostic(code(my_lib::bad_code))]
    Expected(TokenKind),
}

impl ErrorKind {
    fn into_error(self, parser: &Parser) -> miette::Report {
        ParseError {
            kind: self,
            // bad_bit: parser.lex.curr.clone().unwrap().span,
            bad_bit: (1, 1).into(),
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

    #[label("This bit here")]
    bad_bit: SourceSpan,

    #[diagnostic(transparent)]
    kind: ErrorKind,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum Prec {
    #[default]
    Lowest,
    Sum,     // +
    Eq,      // ==
    Cmp,     // > or <
    Product, // *
    Prefix,  // -a or !a
    Call,    // func()
    Index,   // list[0]
    Chain,   // mod.field
}

impl From<&Token> for Prec {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add => Self::Sum,
            TokenKind::LParen => Self::Call,
            _ => Self::Lowest,
        }
    }
}

impl TryFrom<Token> for Op {
    type Error = ParseError;

    fn try_from(value: Token) -> std::result::Result<Self, Self::Error> {
        match value.kind {
            TokenKind::Add => Ok(Self::Add),
            _ => todo!(),
        }
    }
}

pub struct Parser {
    lex: Lexer,
    pub tokens: Vec<Token>,
    curr: Option<Token>,
    next: Option<Token>,
}

impl Parser {
    pub fn new(content: impl ToString) -> Self {
        let mut lex = Lexer::new(content);
        let curr = lex.read_token();
        let next = lex.read_token();
        Self {
            lex,
            curr,
            next,
            tokens: vec![],
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Node>> {
        let mut nodes = vec![];

        loop {
            if self.curr.as_ref().map(|c| c.kind) == Some(TokenKind::Eof) {
                break;
            }

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
        // TODO: whole lot of cloning going on
        let curr = self.next.clone();
        self.curr = self.next.clone();
        self.next = self.lex.read_token();
        self.tokens.push(self.next.clone()?);
        curr
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token> {
        match self.curr.clone() {
            Some(token) if token.kind == expected => {
                self.advance();
                Ok(token)
            }
            _ => Err(ErrorKind::Expected(expected).into_error(self)),
        }
    }

    pub fn node(&mut self) -> Result<Node> {
        let Some(ref curr) = self.curr else {
            return Err(ErrorKind::UnexpectedEOF.into_error(self));
        };

        let node = if curr.kind.is_keyword() {
            Node::Stmnt(self.stmnt()?)
        } else {
            Node::Expr(self.expr(Prec::default())?)
        };

        Ok(node)
    }

    fn ident(&mut self) -> Result<Ident> {
        let token = self.consume(TokenKind::Ident)?;
        Ok(token.text.clone())
    }

    fn ty(&mut self) -> Result<Type> {
        Ok(self.ident()?)
    }

    fn r#fn(&mut self) -> Result<Fn> {
        self.consume(TokenKind::Fn)?;

        let ident = self
            .ident()
            .map_err(|_| miette!("expected ident, got: {:?}", self.curr))?;

        self.consume(TokenKind::LParen)?;
        let mut args = vec![];
        while self.curr.clone().unwrap().kind != TokenKind::RParen {
            args.push(self.fn_arg()?);
        }
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::Arrow)?;
        let return_ty = self.consume(TokenKind::Ident)?;

        let mut nodes = vec![];
        while self.curr.clone().unwrap().kind != TokenKind::End {
            nodes.push(self.node()?);
        }

        self.consume(TokenKind::End)?;

        Ok(Fn {
            ident,
            args,
            return_ty: return_ty.text,
            body: Block { nodes },
        })
    }

    fn r#use(&mut self) -> Result<Use> {
        self.consume(TokenKind::Use)?;
        Ok(Use {
            ident: self.ident()?,
        })
    }

    fn fn_arg(&mut self) -> Result<FnArg> {
        let ident = self.ident()?;
        self.consume(TokenKind::Colon)?;
        let ty = self.ty()?;
        Ok(FnArg { ident, ty })
    }

    fn stmnt(&mut self) -> Result<Stmnt> {
        let Some(ref curr) = self.curr else { panic!() };

        let stmnt = match curr.kind {
            TokenKind::Fn => Stmnt::Fn(self.r#fn()?),
            TokenKind::Use => Stmnt::Use(self.r#use()?),
            TokenKind::Ret => {
                self.advance();
                Stmnt::Ret(self.expr(Prec::default())?)
            }
            _ => panic!("TODO: {}", curr.kind),
            // _ => unreachable!(),
        };
        Ok(stmnt)
    }

    fn infix_expr(&mut self, lhs: Expr) -> Result<Expr> {
        Ok(match self.curr {
            Some(Token {
                kind: TokenKind::Add,
                ..
            }) => {
                let op = self.curr.clone().unwrap().try_into()?;
                self.advance();
                let rhs = self.expr(Prec::default())?; // TODO: prec

                Expr::InfixExpr(InfixExpr {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                })
            }
            _ => panic!(),
        })
    }

    fn call_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LParen)?;

        let mut args = vec![];
        while self.curr.as_ref().unwrap().kind != TokenKind::RParen {
            args.push(self.expr(Prec::Lowest)?);
        }

        self.consume(TokenKind::RParen)?;

        Ok(Expr::CallExpr(CallExpr {
            func: Box::new(expr),
            args,
        }))
    }

    pub fn expr(&mut self, prec: Prec) -> Result<Expr> {
        let Some(ref curr) = self.curr else { panic!() };

        let text = curr.text.clone();

        let mut lhs = match curr.kind {
            TokenKind::Int => Expr::IntLit(text.parse().unwrap()),
            TokenKind::Ident => Expr::Ident(text),
            TokenKind::String => Expr::StringLit(text),
            _ => {
                dbg!(&self.tokens);
                panic!("TODO: {}", curr.kind)
            }
        };

        self.advance();

        // TODO: fix this horribleness
        let Some(curr) = self.curr.clone() else {
            return Ok(lhs);
        };

        while prec < Prec::from(&curr) {
            let Some(curr) = self.curr.clone() else {
                return Ok(lhs);
            };

            if curr.kind == TokenKind::Semicolon {
                self.advance();
                break;
            }

            if curr.kind.is_keyword() {
                break;
            }

            match curr.kind {
                TokenKind::Add | TokenKind::Sub => lhs = self.infix_expr(lhs)?,
                TokenKind::LParen => lhs = self.call_expr(lhs)?,
                _ => panic!("TODO: {:?} text: {}", curr.kind, curr.text),
            };
        }

        Ok(lhs)
    }
}
