use crate::ast::{Expr, Fn, Identifier, InfixExpr, Node, Op, Stmnt};
use crate::lexer::{Lexer, Token, TokenKind};

use miette::{Diagnostic, IntoDiagnostic, NamedSource, Result, SourceOffset, SourceSpan};
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
    // #[error(transparent)]
    // #[diagnostic(code(my_lib::io_error))]
    // IoError(#[from] std::io::Error),

    // #[error("Oops it blew up")]
    // #[diagnostic(code(my_lib::bad_code))]
    // BadThingHappened,

    // #[error(transparent)]
    // // Use `#[diagnostic(transparent)]` to wrap another [`Diagnostic`]. You won't see labels otherwise
    // #[diagnostic(transparent)]
    // AnotherError(#[from] AnotherError),
}

impl ErrorKind {
    fn into_error(self, parser: &Parser) -> miette::Report {
        ParseError {
            kind: self,
            // TODO: get location from lexer position
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
pub enum Precedence {
    #[default]
    Lowest,
    Sum,     // +
    Eq,      // ==
    Cmp,     // > or <
    Product, // *
    Prefix,  // -a or !a
             // Call,    // func()
             // Index,   // list[0]
             // Chain,   // mod.field
}

impl From<&Token> for Precedence {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add => Self::Sum,
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
    curr: Option<Token>,
    next: Option<Token>,
}

impl Parser {
    pub fn new(content: impl ToString) -> Self {
        let mut lex = Lexer::new(content);
        let curr = lex.read_token();
        let next = lex.read_token();
        Self { lex, curr, next }
    }

    pub fn parse(&mut self) -> Result<Vec<Node>> {
        let mut nodes = vec![];

        // TODO: this is a dirty solution
        loop {
            match self.node() {
                Ok(node) => nodes.push(node),
                Err(err) => {
                    err.downcast()?;
                    // eprintln!("parse node error: {err:#}");
                    break;
                }
            }
        }

        Ok(nodes)
    }

    fn advance(&mut self) -> Option<Token> {
        let curr = self.next.clone();
        self.curr = self.next.clone();
        self.next = self.lex.read_token();
        curr
    }

    fn consume(&mut self, expected: TokenKind) -> Result<&Token> {
        match self.curr {
            Some(ref token) if token.kind == expected => Ok(token),
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
            Node::Expr(self.expr(Precedence::default())?)
        };

        Ok(node)
    }

    fn ident(&mut self) -> Result<Identifier> {
        let token = self.consume(TokenKind::Ident)?;
        Ok(token.text.clone())
    }

    fn func(&mut self) -> Result<Fn> {
        self.consume(TokenKind::Fn)?;
        let ident = self.ident()?;

        dbg!("hello func", self.curr.clone());
        todo!()
    }

    fn stmnt(&mut self) -> Result<Stmnt> {
        let Some(ref curr) = self.curr else { panic!() };

        let stmnt = match curr.kind {
            TokenKind::Fn => Stmnt::Fn(self.func()?),
            _ => unimplemented!(),
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
                let rhs = self.expr(Precedence::default())?; // TODO: prec

                Expr::BinOp(InfixExpr {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                })
            }
            _ => panic!(),
        })
    }

    pub fn expr(&mut self, prec: Precedence) -> Result<Expr> {
        let Some(ref curr) = self.curr else { panic!() };

        let mut lhs = match curr.kind {
            TokenKind::Int => Expr::IntLit(curr.text.clone().parse().unwrap()),
            _ => todo!(),
        };

        self.advance();

        // TODO: fix this horribleness
        let Some(curr) = self.curr.clone() else {
            return Ok(lhs);
        };

        while prec < Precedence::from(&curr) {
            let Some(curr) = self.curr.clone() else {
                return Ok(lhs);
            };

            match curr.kind {
                TokenKind::Add => lhs = self.infix_expr(lhs)?,
                _ => todo!(),
            };
        }

        Ok(lhs)
    }
}
