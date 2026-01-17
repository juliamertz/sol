use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;

use crate::lexer::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Eof,
    Newline,

    // Literals
    Int,
    String,
    Ident,

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
    Fn,
    Ret,
    If,
    Then,
    Else,
    End,
    Use,
    Extern,
    Struct,

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

impl TokenKind {
    #[allow(dead_code)]
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Let
                | TokenKind::Fn
                | TokenKind::If
                | TokenKind::Then
                | TokenKind::Else
                | TokenKind::End
                | TokenKind::Ret
                | TokenKind::Use
                | TokenKind::Extern
                | TokenKind::Struct
        )
    }

    // expression parsing should stop if this token is encountered
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            TokenKind::Eof
                | TokenKind::End
                | TokenKind::Semicolon
                | TokenKind::Comma
                | TokenKind::RBracket
                | TokenKind::RParen
                | TokenKind::RSquirly
                | TokenKind::Then
                | TokenKind::Else
        )
    }

    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            TokenKind::Eq
                | TokenKind::Assign
                | TokenKind::Add
                | TokenKind::Sub
                | TokenKind::Asterisk
                | TokenKind::Slash
                | TokenKind::Arrow
                | TokenKind::RAngle
                | TokenKind::LAngle
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Ampersand
        )
    }

    pub fn is_prefix_operator(&self) -> bool {
        matches!(self, TokenKind::Bang | TokenKind::Sub)
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub static KEYWORD_LOOKUP: LazyLock<HashMap<&'static str, TokenKind>> = LazyLock::new(|| {
    [
        ("let", TokenKind::Let),
        ("func", TokenKind::Fn),
        ("return", TokenKind::Ret),
        ("if", TokenKind::If),
        ("else", TokenKind::Else),
        ("then", TokenKind::Then),
        ("end", TokenKind::End),
        ("use", TokenKind::Use),
        ("and", TokenKind::And),
        ("or", TokenKind::Or),
        ("extern", TokenKind::Extern),
        ("struct", TokenKind::Struct),
    ]
    .into_iter()
    .collect()
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, text: impl ToString, start_pos: usize) -> Self {
        let text = text.to_string();
        Self {
            kind,
            text: text.to_string(),
            span: (start_pos, text.len()).into(),
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

impl AsRef<Token> for Token {
    fn as_ref(&self) -> &Token {
        self
    }
}
