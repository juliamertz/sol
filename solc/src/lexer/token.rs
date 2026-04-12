use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;

use strum::EnumIs;

use crate::lexer::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIs)]
pub enum TokenKind {
    Eof,
    Newline,

    // Literals
    Int,
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

    // Operators
    Eq,
    Assign,
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
    #[allow(dead_code)]
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
        )
    }

    // expression parsing should stop if this token is encountered
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

pub static KEYWORD_LOOKUP: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {
    [
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
    ]
    .into_iter()
    .collect()
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'src> {
    pub kind: Kind,
    pub text: &'src str,
    pub span: Span,
}

impl Token<'_> {
    pub fn owned(&self) -> OwnedToken {
        OwnedToken {
            kind: self.kind,
            text: self.text.to_string(),
            span: self.span,
        }
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }
}

#[derive(Debug, Clone)]
pub struct OwnedToken {
    pub kind: Kind,
    pub text: String,
    pub span: Span,
}

impl<'src> Token<'src> {
    pub fn new(kind: Kind, text: &'src str, start_pos: usize) -> Self {
        Self {
            kind,
            text,
            span: (start_pos, text.len()).into(),
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}
