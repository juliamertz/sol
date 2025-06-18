use miette::{Diagnostic, NamedSource, Result, SourceSpan, miette};
use thiserror::Error;

use crate::ast::*;
use crate::lexer::{Lexer, Token, TokenKind};

#[derive(Error, Diagnostic, Debug)]
pub enum ErrorKind {
    #[error("expected token {0}")]
    Expected(TokenKind),

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
    Prefix, // -a, !a or &a
    Call, // func()
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
            TokenKind::Bang | TokenKind::Sub | TokenKind::Ampersand => Self::Prefix,
            _ => Self::Lowest,
        }
    }
}

impl TryFrom<Token> for Op {
    type Error = ErrorKind;

    fn try_from(token: Token) -> std::result::Result<Self, Self::Error> {
        match token.kind {
            TokenKind::Add => Ok(Self::Add),
            TokenKind::Sub => Ok(Self::Sub),
            TokenKind::Eq => Ok(Self::Eq),
            TokenKind::Asterisk => Ok(Self::Mul),
            TokenKind::Slash => Ok(Self::Div),
            TokenKind::LAngle => Ok(Self::Lt),
            TokenKind::RAngle => Ok(Self::Gt),
            TokenKind::And => Ok(Self::And),
            TokenKind::Or => Ok(Self::Or),
            TokenKind::Dot => Ok(Self::Chain),
            _ => Err(ErrorKind::InvalidOperator { token }),
        }
    }
}

pub struct Parser {
    lex: Lexer,
    pub tokens: Vec<Token>,
    curr: Token,
    next: Option<Token>,
}

impl Parser {
    pub fn new(content: impl ToString) -> Self {
        let mut lex = Lexer::new(content);
        let curr = lex
            .read_token()
            .unwrap_or(Token::new(TokenKind::Eof, "", lex.pos));
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
            if self.curr.kind == TokenKind::Eof {
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

        if let Some(next) = self.next.clone() {
            self.curr = next;
        }
        // self.curr = self.next.clone();
        self.next = self.lex.read_token();
        self.tokens.push(self.next.clone()?);
        curr
    }

    fn consume(&mut self, expected: TokenKind) -> Result<Token> {
        if self.curr.kind != expected {
            return Err(ErrorKind::Expected(expected).into_error(self));
        }

        let tok = self.curr.clone();
        self.advance();
        Ok(tok)
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

        let _ = self.consume(TokenKind::Semicolon);

        Ok(node)
    }

    fn block(&mut self) -> Result<Block> {
        let mut nodes = vec![];
        loop {
            if self.curr.kind.is_terminator() {
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
                    token: curr.clone(),
                }
                .into_error(self));
            }
        };
        Ok(ty)
    }

    fn r#fn(&mut self) -> Result<Fn> {
        let is_extern = self.curr.kind == TokenKind::Extern;
        if is_extern {
            self.advance();
        }

        self.consume(TokenKind::Fn).unwrap();

        let ident = self
            .ident()
            .map_err(|_| miette!("expected ident, got: {:?}", self.curr))?;

        self.consume(TokenKind::LParen)?;
        let mut args = vec![];
        while self.curr.kind != TokenKind::RParen {
            args.push(self.arg_type()?);
        }
        self.consume(TokenKind::RParen)?;

        self.consume(TokenKind::Arrow)?;
        let return_ty = self.ty()?;

        let body = if self.curr.kind.is_terminator() {
            None
        } else {
            let mut nodes = vec![];
            while self.curr.kind != TokenKind::End {
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

    fn arg_type(&mut self) -> Result<(Ident, Type)> {
        let ident = self.ident()?;
        self.consume(TokenKind::Colon)?;
        let ty = self.ty()?;
        Ok((ident, ty))
    }

    fn arg_value(&mut self) -> Result<(Ident, Expr)> {
        let ident = self.ident()?;
        self.consume(TokenKind::Colon)?;
        let expr = self.expr(Prec::Lowest)?;
        Ok((ident, expr))
    }

    fn arg_types(&mut self) -> Result<Vec<(Ident, Type)>> {
        let mut args = vec![];

        loop {
            if self.curr.kind == TokenKind::Comma {
                self.advance();
            }

            if self.curr.kind == TokenKind::Comma {
                self.advance();
            } else if self.curr.kind.is_terminator() {
                break;
            }

            args.push(self.arg_type()?);
        }

        Ok(args)
    }

    fn arg_values(&mut self) -> Result<Vec<(Ident, Expr)>> {
        let mut args = vec![];

        loop {
            if self.curr.kind == TokenKind::Comma {
                self.advance();
            }

            if self.curr.kind == TokenKind::Comma {
                self.advance();
            } else if self.curr.kind.is_terminator() {
                break;
            }

            args.push(self.arg_value()?);
        }

        Ok(args)
    }

    fn stmnt(&mut self) -> Result<Stmnt> {
        let stmnt = match self.curr.kind {
            TokenKind::Fn | TokenKind::Extern => Stmnt::Fn(self.r#fn()?),
            TokenKind::Use => Stmnt::Use(self.r#use()?),
            TokenKind::Let => Stmnt::Let(self.r#let()?),
            TokenKind::Struct => Stmnt::StructDef(self.struct_def()?),
            TokenKind::Ret => {
                self.advance();
                let expr = self.expr(Prec::default())?;
                self.consume(TokenKind::Semicolon)?;
                Stmnt::Ret(Ret { val: expr })
            }
            _ => panic!("TODO: {}", self.curr.kind),
            // _ => unreachable!(),
        };

        Ok(stmnt)
    }

    fn r#let(&mut self) -> Result<Let> {
        self.consume(TokenKind::Let)?;
        let ident = self.ident()?;

        let mut ty = None;
        if self.curr.kind == TokenKind::Colon {
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
        let alternative = if self.curr.kind == TokenKind::Else {
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

    fn prefix_expr(&mut self, op: Op) -> Result<Expr> {
        let rhs = self.expr(Prec::default())?;
        Ok(Expr::Prefix(PrefixExpr {
            op,
            rhs: Box::new(rhs),
        }))
    }

    fn infix_expr(&mut self, lhs: Expr) -> Result<Expr> {
        if !self.curr.kind.is_operator() {
            panic!("invalid operator");
        }
        let op: Op = self.curr.to_owned().try_into()?;
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
        let text = self.curr.text.clone();
        let mut lhs = match self.curr.kind {
            TokenKind::Int => Expr::IntLit(text.parse().unwrap()),
            TokenKind::Ident => Expr::Ident(text),
            TokenKind::String => Expr::StringLit(text),
            TokenKind::If => Expr::If(self.r#if()?),
            TokenKind::LBracket => Expr::List(self.list()?),

            _ => panic!("{:?}", ErrorKind::Todo(self.curr.clone()).into_error(self)),
        };

        self.advance();

        if self.curr.kind == TokenKind::Eof {
            return Ok(lhs);
        }

        while prec <= Prec::from(&self.curr) {
            if self.curr.kind.is_terminator() {
                break;
            }

            dbg!(&lhs);

            match self.curr.kind {
                kind if kind.is_operator() => {
                    lhs = self.infix_expr(lhs)?;
                }
                TokenKind::LParen => {
                    lhs = self.call_expr(lhs)?;
                }
                TokenKind::LSquirly => {
                    lhs = self.struct_constructor(lhs)?;
                }
                _ => todo!("kind: {}", self.curr.kind),
            }
        }

        Ok(lhs)
    }

    fn expr_list(&mut self) -> Result<Vec<Expr>> {
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
        self.consume(TokenKind::LBracket)?;
        let items = self.expr_list()?;
        self.consume(TokenKind::RBracket)?;
        Ok(List { items })
    }

    fn struct_def(&mut self) -> Result<StructDef> {
        self.consume(TokenKind::Struct)?;
        let ident = self.ident()?;
        self.consume(TokenKind::Assign)?;
        let fields = self.arg_types()?;
        self.consume(TokenKind::End)?;
        Ok(StructDef { ident, fields })
    }

    fn struct_constructor(&mut self, lhs: Expr) -> Result<Expr> {
        // TODO: helper function to make extracting ident from expr easier

        let Expr::Ident(ident) = lhs else {
            return Err(ErrorKind::Expected(TokenKind::Ident).into_error(self));
        };

        self.consume(TokenKind::LSquirly)?;
        let fields = self.arg_values()?;
        self.consume(TokenKind::RSquirly)?;
        Ok(Expr::StructConstructor(StructConstructor { ident, fields }))
    }
}
