use crate::lexer::{Token, TokenKind};

/// Precedence of a token produced by the lexer
///
/// this dictates the order in which expressions are traversed in the parser
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Default)]
pub enum Prec {
    #[default]
    /// lowest precedence, this is the default
    Lowest,
    /// `a = 10`
    Assign,
    /// `&&` or `||` lower precedence than equality
    AndOr,
    /// `==` or `!=`
    Eq,
    /// `>` or `<`
    Cmp,
    /// `+`
    Sum,
    /// `*`
    Product,
    /// `-a`, `!a` or `&a`
    Unary,
    /// `func()`
    Call,
    /// `Point { x : 10, y : 5 }`
    Construct,
    /// `list[0]`
    Index,
    /// `mod.field`
    Chain,
}

impl From<&Token<'_>> for Prec {
    fn from(token: &Token) -> Self {
        match token.kind {
            TokenKind::Add | TokenKind::Sub => Self::Sum,
            TokenKind::Assign => Self::Assign,
            TokenKind::Eq | TokenKind::Ne => Self::Eq,
            TokenKind::LParen => Self::Call,
            TokenKind::LSquirly => Self::Construct,
            TokenKind::LBracket => Self::Index,
            TokenKind::LAngle | TokenKind::RAngle => Self::Cmp,
            TokenKind::Asterisk => Self::Product,
            TokenKind::And | TokenKind::Or => Self::AndOr,
            TokenKind::Dot => Self::Chain,
            TokenKind::Bang | TokenKind::Ampersand => Self::Unary,
            _ => Self::Lowest,
        }
    }
}
