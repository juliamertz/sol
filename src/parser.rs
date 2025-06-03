use crate::ast::{BinOp, Expr, Op};
use crate::lexer::{Lexer, Token};

use miette::{Diagnostic, NamedSource, Result, SourceOffset, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum ParseErrorKind {
    #[error("Illegal token")]
    #[diagnostic(code(my_lib::bad_code))]
    Illegal,

    #[error("Unexpected EOF")]
    #[diagnostic(code(my_lib::bad_code))]
    UnexpectedEOF,

    #[error("Invalid operator")]
    #[diagnostic(code(my_lib::bad_code))]
    InvalidOperator,
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

#[derive(Error, Debug, Diagnostic)]
#[error("oops!")]
#[diagnostic(code(oops::my::bad), help("try doing it better next time?"))]
pub struct ParseError {
    #[source_code]
    src: NamedSource<String>,

    #[label("This bit here")]
    bad_bit: SourceSpan,

    kind: ParseErrorKind,
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
        match token {
            Token::Add => Self::Sum,
            _ => Self::Lowest,
        }
    }
}

impl TryFrom<Token> for Op {
    type Error = ParseError;

    fn try_from(value: Token) -> std::result::Result<Self, Self::Error> {
        match value {
            Token::Add => Ok(Self::Add),
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

    pub fn parse(self) -> Result<Vec<Token>> {
        Ok(vec![])
    }

    fn advance(&mut self) -> Option<Token> {
        let curr = self.next.clone();
        self.curr = self.next.clone();
        self.next = self.lex.read_token();
        curr
    }

    fn infix_expr(&mut self, lhs: Expr) -> Result<Expr> {
        Ok(match self.curr {
            Some(Token::Add) => {
                let op = self.curr.clone().unwrap().try_into()?;
                self.advance();
                let rhs = self.expr(Precedence::default())?; // TODO: prec

                Expr::BinOp(BinOp {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                })
            }
            _ => panic!(),
        })
    }

    pub fn expr(&mut self, prec: Precedence) -> Result<Expr> {
        let mut lhs = match self.curr.clone().unwrap() {
            Token::Int(val) => Expr::IntLit(val),
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

            match curr {
                Token::Add => lhs = self.infix_expr(lhs)?,
                _ => todo!(),
            };
        }

        Ok(lhs)
    }
}
