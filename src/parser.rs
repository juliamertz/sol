use miette::{Diagnostic, NamedSource, Result, SourceSpan, miette};
use thiserror::Error;

use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};

#[derive(Error, Diagnostic, Debug)]
pub enum ErrorKind {
    #[error("Unexpected EOF")]
    UnexpectedEOF,

    #[error("expected token {0}")]
    Expected(TokenKind),

    #[diagnostic(code(lib::bad_code))]
    #[error("invalid type: {}", token.text)]
    InvalidType { token: Token },

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
    // Prefix, // -a or !a
    Call, // func()
          // Index,  // list[0]
          // Chain,  // mod.field
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
            TokenKind::LAngle => Ok(Self::Lt),
            TokenKind::RAngle => Ok(Self::Gt),
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

    // fn kind(&self) -> &TokenKind {
    //     self.curr.as_ref().map(|token| token.kind)
    // }
    //
    fn match_kind(&self, kind: TokenKind) -> bool {
        let actual = self.curr.as_ref().map(|token| token.kind);
        let expected = Some(kind);
        matches!(expected, actual)
    }

    pub fn node(&mut self) -> Result<Node> {
        let Some(ref curr) = self.curr else {
            return Err(ErrorKind::UnexpectedEOF.into_error(self));
        };

        let node = if matches!(
            curr.kind,
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

        let _ = self.consume(TokenKind::Semicolon);

        Ok(node)
    }

    fn block(&mut self) -> Result<Block> {
        let mut nodes = vec![];
        while let Some(ref curr) = self.curr {
            if curr.kind.is_terminator() {
                break;
            }
            // if matches!(curr.kind, TokenKind::End | TokenKind::Eof) {
            //     self.advance();
            //     break;
            // }
            nodes.push(self.node()?);
        }

        Ok(Block { nodes })
    }

    fn ident(&mut self) -> Result<Ident> {
        let token = self.consume(TokenKind::Ident)?;
        Ok(token.text.clone())
    }

    fn ty(&mut self) -> Result<Type> {
        let curr = self.curr.clone();
        let ident = self.ident()?;
        let ty = match ident.as_str() {
            "Int" => Type::Int,
            "Bool" => Type::Bool,
            "Str" => Type::Str,
            "List" => {
                self.consume(TokenKind::LAngle)?;
                let inner = self.ty()?;
                self.consume(TokenKind::RAngle)?;
                Type::List(Box::new(inner))
            }
            _ => {
                return Err(ErrorKind::InvalidType {
                    token: curr.clone().unwrap(),
                }
                .into_error(self));
            }
        };
        Ok(ty)
    }

    fn r#fn(&mut self) -> Result<Fn> {
        let is_extern = self.curr.as_ref().map(|t| t.kind) == Some(TokenKind::Extern);
        if is_extern {
            self.advance();
        }

        self.consume(TokenKind::Fn).unwrap();

        let ident = self
            .ident()
            .map_err(|_| miette!("expected ident, got: {:?}", self.curr))?;

        self.consume(TokenKind::LParen)?;
        let mut args = vec![];
        while self.curr.clone().unwrap().kind != TokenKind::RParen {
            args.push(self.typed_arg()?);
        }
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::Arrow)?;
        let return_ty = self.ty()?;

        let body = if self
            .curr
            .as_ref()
            .map(|tok| tok.kind.is_terminator())
            .unwrap_or(true)
        {
            None
        } else {
            let mut nodes = vec![];
            while self.curr.clone().unwrap().kind != TokenKind::End {
                nodes.push(self.node()?);
            }

            self.consume(TokenKind::End)?;

            Some(Block { nodes })
        };

        Ok(Fn {
            is_extern,
            name: ident,
            args,
            return_ty,
            body,
        })
    }

    fn r#use(&mut self) -> Result<Use> {
        self.consume(TokenKind::Use)?;
        Ok(Use {
            ident: self.ident()?,
        })
    }

    fn typed_arg(&mut self) -> Result<TypedArg> {
        dbg!("sup");
        let ident = self.ident()?;
        dbg!(&ident);
        self.consume(TokenKind::Colon)?;
        let ty = self.ty()?;
        Ok(TypedArg { ident, ty })
    }

    fn typed_args(&mut self) -> Result<Vec<TypedArg>> {
        let mut args = vec![];

        loop {
            if self.match_kind(TokenKind::Comma) {
                self.advance();
            }

            let Some(kind) = self.curr.as_ref().map(|t| t.kind) else {
                break;
            };

            dbg!(&kind);

            if kind == TokenKind::Comma {
                self.advance();
            } else if kind.is_terminator() {
                break;
            }

            let arg = self.typed_arg()?;

            dbg!(&arg);

            dbg!(&self.curr);
        }

        Ok(args)
    }

    fn stmnt(&mut self) -> Result<Stmnt> {
        let Some(ref curr) = self.curr else { panic!() };

        let stmnt = match curr.kind {
            TokenKind::Fn | TokenKind::Extern => Stmnt::Fn(self.r#fn()?),
            TokenKind::Use => Stmnt::Use(self.r#use()?),
            TokenKind::Let => Stmnt::Let(self.r#let()?),
            TokenKind::Struct => Stmnt::Struct(self.r#struct()?),
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

        let mut ty = None;
        if let Some(TokenKind::Colon) = self.curr.as_ref().map(|t| t.kind) {
            self.consume(TokenKind::Colon)?;
            ty = Some(self.ty()?);
        }

        self.consume(TokenKind::Assign)?;
        let val = Some(self.expr(Prec::Lowest)?);
        Ok(Let {
            name: ident,
            ty,
            val,
        })
    }

    fn r#if(&mut self) -> Result<If> {
        self.consume(TokenKind::If)?;
        let condition = self.expr(Prec::Lowest)?;
        self.consume(TokenKind::Then)?;
        let consequence = self.block()?;
        let alternative = if self.curr.as_ref().map(|t| t.kind) == Some(TokenKind::Else) {
            self.advance();
            Some(self.block()?)
        } else {
            None
        };

        // TODO: end?
        Ok(If {
            condition: Box::new(condition),
            consequence,
            alternative,
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

        Ok(Expr::Infix(InfixExpr {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
        }))
    }

    fn call_expr(&mut self, expr: Expr) -> Result<Expr> {
        self.consume(TokenKind::LParen)?;
        let args = self.expr_list()?;
        self.consume(TokenKind::RParen)?;

        Ok(Expr::Call(CallExpr {
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
            TokenKind::LBracket => Expr::List(self.list()?),

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

            if curr.kind.is_terminator() {
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

    fn r#struct(&mut self) -> Result<Struct> {
        self.consume(TokenKind::Struct)?;

        let ident = self.ident()?;

        self.consume(TokenKind::Assign)?;

        let fields = self.typed_args()?;

        dbg!(&ident, &fields);

        todo!()
    }
}
