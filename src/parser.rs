use miette::{Diagnostic, NamedSource, Result, SourceSpan, miette};
use thiserror::Error;

use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};

#[derive(Error, Diagnostic, Debug)]
pub enum ErrorKind {
    #[error("Illegal token")]
    #[diagnostic(code(my_lib::bad_code))]
    Illegal,

    #[error("Unexpected EOF")]
    #[diagnostic(code(my_lib::bad_code))]
    UnexpectedEOF,

    #[error("expected token {0}")]
    #[diagnostic(code(my_lib::bad_code))]
    Expected(TokenKind),

    #[error("unhandled token: {0:?}")]
    #[diagnostic(code(my_lib::bad_code))]
    Todo(Token),
}

impl ErrorKind {
    fn into_error(self, parser: &Parser) -> miette::Report {
        let span = match self {
            ErrorKind::Todo(ref token) => token.span,
            _ => (parser.lex.pos, 1).into(),
        };

        ParseError {
            kind: self,
            bad_bit: span,
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
    AndOr,
    Prefix,  // -a or !a
    Call,    // func()
    Index,   // list[0]
    Chain,   // mod.field
}

impl From<&Token> for Prec {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add | TokenKind::Sub => Self::Sum,
            TokenKind::Eq => Self::Eq,
            TokenKind::LParen => Self::Call,
            TokenKind::Lt | TokenKind::Gt => Self::Cmp,
            TokenKind::Asterisk => Self::Product,
            TokenKind::And | TokenKind::Or => Self::AndOr,
            _ => Self::Lowest,
        }
    }
}

impl TryFrom<Token> for Op {
    type Error = ParseError;

    fn try_from(value: Token) -> std::result::Result<Self, Self::Error> {
        match value.kind {
            TokenKind::Add => Ok(Self::Add),
            TokenKind::Sub => Ok(Self::Sub),
            TokenKind::Eq => Ok(Self::Eq),
            TokenKind::Asterisk => Ok(Self::Mul),
            TokenKind::Slash => Ok(Self::Div),
            TokenKind::Lt => Ok(Self::Lt),
            TokenKind::Gt => Ok(Self::Gt),
            TokenKind::And => Ok(Self::And),
            TokenKind::Or => Ok(Self::Or),
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

        let node = if matches!(
            curr.kind,
            TokenKind::Ret | TokenKind::Use | TokenKind::Fn | TokenKind::Let
        ) {
            Node::Stmnt(self.stmnt()?)
        } else {
            Node::Expr(self.expr(Prec::default())?)
        };

        let _ = self.consume(TokenKind::Semicolon);

        Ok(node)
    }

    fn block(&mut self) -> Result<Block> {
        let mut nodes = vec![];
        while let Some(ref curr) = self.curr {
            if matches!(curr.kind, TokenKind::End | TokenKind::Eof) {
                self.advance();
                break;
            }
            nodes.push(self.node()?);
        }

        Ok(Block { nodes })
    }

    fn ident(&mut self) -> Result<Ident> {
        let token = self.consume(TokenKind::Ident)?;
        Ok(token.text.clone())
    }

    fn ty(&mut self) -> Result<Ty> {
        self.ident()
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

    // fn ret(&mut self) -> Result<

    fn stmnt(&mut self) -> Result<Stmnt> {
        let Some(ref curr) = self.curr else { panic!() };

        let stmnt = match curr.kind {
            TokenKind::Fn => Stmnt::Fn(self.r#fn()?),
            TokenKind::Use => Stmnt::Use(self.r#use()?),
            TokenKind::Let => Stmnt::Let(self.r#let()?),
            TokenKind::Ret => {
                self.advance();
                let expr = self.expr(Prec::default())?;
                self.consume(TokenKind::Semicolon)?;
                Stmnt::Ret(Ret { val: expr })
            }
            _ => panic!("TODO: {}", curr.kind),
            // _ => unreachable!(),
        };

        Ok(stmnt)
    }

    fn r#let(&mut self) -> Result<Let> {
        self.consume(TokenKind::Let)?;
        let ident = self.ident()?;
        self.consume(TokenKind::Colon)?;
        let ty = self.ty()?;
        self.consume(TokenKind::Assign)?;
        let val = Some(self.expr(Prec::Lowest)?);
        Ok(Let { ident, ty, val })
    }

    fn r#if(&mut self) -> Result<If> {
        self.consume(TokenKind::If)?;
        let condition = self.expr(Prec::Lowest)?;
        self.consume(TokenKind::Then)?;
        let consequence = self.block()?;
        Ok(If {
            condition: Box::new(condition),
            consequence,
        })
    }

    fn infix_expr(&mut self, lhs: Expr) -> Result<Expr> {
        let Some(ref curr) = self.curr else {
            return Err(ErrorKind::UnexpectedEOF.into_error(self));
        };

        if !curr.kind.is_operator() {
            panic!("invalid operator");
        }
        let op: Op = curr.to_owned().try_into()?;
        self.advance();

        let rhs = self.expr(Prec::default())?; // TODO: prec

        Ok(Expr::InfixExpr(InfixExpr {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
        }))
    }

    fn call_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LParen)?;
        let args = self.expr_list()?;
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
            TokenKind::If => Expr::If(self.r#if()?),

            _ => panic!("{:?}", ErrorKind::Todo(curr.clone()).into_error(self)),
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

            if matches!(
                curr.kind,
                TokenKind::RParen
                    | TokenKind::Eof
                    | TokenKind::Then
                    | TokenKind::Semicolon
                    | TokenKind::Comma
                    | TokenKind::End
            ) {
                break;
            }

            if curr.kind.is_operator() {
                lhs = self.infix_expr(lhs)?;
            } else if curr.kind == TokenKind::LParen {
                lhs = self.call_expr(lhs)?;
            } else {
                panic!("TODO: {:?} text: {}", curr.kind, curr.text);
            }
        }

        Ok(lhs)
    }

    fn expr_list(&mut self) -> Result<Vec<Expr>> {
        let head = self.expr(Prec::Lowest)?;

        let mut tail = vec![];
        tail.push(head);

        while let Some(ref token) = self.curr {
            if token.kind != TokenKind::Comma {
                break;
            }

            self.consume(TokenKind::Comma)?;
            tail.push(self.expr(Prec::Lowest)?);
        }

        Ok(tail)
    }

    fn list(&mut self) -> Result<List> {
        self.consume(TokenKind::LBracket)?;
        let items = self.expr_list()?;
        self.consume(TokenKind::RBracket)?;
        Ok(List { items })
    }
}
