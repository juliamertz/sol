use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;

use strum::EnumIs;

use crate::lexer::source::Span;
use crate::lexer::num::{ReadNumber};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIs)]
pub enum TokenKind {
    Eof,
    Newline,

    // Literals
    Num(ReadNumber),
    String,
    Ident,
    True,
    False,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    LSquirly,
    RSquirly,
    Dot,
    Comma,
    Colon,
    Semicolon,

    // Keywords
    Let,
    Mut,
    Fn,
    Ret,
    If,
    Then,
    Else,
    End,
    Use,
    Extern,
    Struct,
    Impl,
    Variadic,
    While,
    Do,

    // Operators
    Assign,
    Eq,
    Ne,
    Add,
    Sub,
    Asterisk,
    Slash,
    Arrow,
    And,
    Or,
    Bang,
    Ampersand,
}

use TokenKind as Kind;

impl Kind {
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Kind::Let
                | Kind::Fn
                | Kind::If
                | Kind::Then
                | Kind::Else
                | Kind::End
                | Kind::Ret
                | Kind::Use
                | Kind::Extern
                | Kind::Struct
                | Kind::Impl
                | Kind::True
                | Kind::False
                | Kind::Variadic
                | Kind::Mut
                | Kind::While
                | Kind::Do
        )
    }

    /// a token that is a terminator indicates whether expression parsing should stop or can continue
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Kind::Eof
                | Kind::End
                | Kind::Semicolon
                | Kind::Comma
                | Kind::RBracket
                | Kind::RParen
                | Kind::RSquirly
                | Kind::Then
                | Kind::Else
        )
    }

    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Kind::Eq
                | Kind::Ne
                | Kind::Add
                | Kind::Sub
                | Kind::Asterisk
                | Kind::Slash
                | Kind::Arrow
                | Kind::RAngle
                | Kind::LAngle
                | Kind::And
                | Kind::Or
                | Kind::Ampersand
        )
    }

    pub fn is_unary_op(&self) -> bool {
        matches!(self, Kind::Bang | Kind::Sub)
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

type TokenLookup = HashMap<&'static str, Kind>;

pub(super) static KEYWORD_LOOKUP: LazyLock<TokenLookup> = LazyLock::new(|| {
    TokenLookup::from([
        ("let", Kind::Let),
        ("mut", Kind::Mut),
        ("func", Kind::Fn),
        ("return", Kind::Ret),
        ("if", Kind::If),
        ("else", Kind::Else),
        ("then", Kind::Then),
        ("end", Kind::End),
        ("use", Kind::Use),
        ("and", Kind::And),
        ("or", Kind::Or),
        ("extern", Kind::Extern),
        ("struct", Kind::Struct),
        ("impl", Kind::Impl),
        ("true", Kind::True),
        ("false", Kind::False),
        ("variadic", Kind::Variadic),
        ("while", Kind::While),
        ("do", Kind::Do),
    ])
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'src> {
    pub kind: Kind,
    pub text: Cow<'src, str>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct OwnedToken {
    pub kind: Kind,
    pub text: String,
    pub span: Span,
}

impl<'src> Token<'src> {
    pub fn new(kind: Kind, text: impl Into<Cow<'src, str>>, start_pos: usize) -> Self {
        let text = text.into();
        Self {
            span: (start_pos, text.len()).into(),
            kind,
            text,
        }
    }

    pub fn to_owned(&self) -> OwnedToken {
        OwnedToken {
            kind: self.kind,
            text: self.text.to_string(),
            span: self.span,
        }
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    pub fn span(&self) -> Span {
        self.span
    }
}
