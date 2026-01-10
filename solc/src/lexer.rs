use std::{collections::HashMap, fmt::Display};

use lazy_static::lazy_static;

pub type Span = miette::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Serialize, serde::Deserialize))]
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
                | TokenKind::Dot
                | TokenKind::Ampersand
        )
    }
}

lazy_static! {
    static ref KEYWORD_LOOKUP: HashMap<&'static str, TokenKind> = [
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
    .iter()
    .cloned()
    .collect();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
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

#[derive(Debug)]
pub struct Lexer {
    pub content: String,
    pub pos: usize,
    pub eof: bool,
}

impl Lexer {
    pub fn new(content: impl ToString) -> Self {
        Self {
            content: content.to_string(),
            pos: 0,
            eof: false,
        }
    }

    pub fn advance(&mut self) -> Option<char> {
        self.pos += 1;
        self.content.chars().nth(self.pos)
    }

    pub fn curr(&self) -> Option<char> {
        self.content.chars().nth(self.pos)
    }

    pub fn peek(&self) -> Option<char> {
        self.content.chars().nth(self.pos + 1)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.curr() {
            if ch.is_ascii_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_while<F>(&mut self, condition: F) -> &str
    where
        F: Fn(char) -> bool,
    {
        let start = self.pos;

        while let Some(ch) = self.curr() {
            if condition(ch) {
                self.advance();
            } else {
                break;
            }
        }

        &self.content[start..self.pos]
    }

    fn read_string(&mut self) -> &str {
        assert_eq!(self.curr(), Some('"'),);

        self.advance();

        (self.read_while(|ch| ch != '"')) as _
    }

    pub fn read_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        // Dirty little hack to return EOF as last token
        if self.curr().is_none() && !self.eof {
            self.eof = true;
            return Some(Token::new(TokenKind::Eof, "", self.pos));
        }

        let start = self.pos;

        let token = match self.curr()? {
            '"' => Token::new(TokenKind::String, self.read_string().to_string(), start),
            '+' => Token::new(TokenKind::Add, "+", start),
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Eq, "==", start)
                } else {
                    Token::new(TokenKind::Assign, "=", start)
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token::new(TokenKind::Arrow, "->", start)
                } else if self.peek() == Some('-') {
                    self.read_while(|ch| ch != '\n');
                    self.read_token()?
                } else {
                    Token::new(TokenKind::Sub, "-", start)
                }
            }
            '!' => Token::new(TokenKind::Bang, "!", start),
            '*' => Token::new(TokenKind::Asterisk, "*", start),
            '/' => Token::new(TokenKind::Slash, "/", start),
            '&' => Token::new(TokenKind::Ampersand, "&", start),
            '(' => Token::new(TokenKind::LParen, "(", start),
            ')' => Token::new(TokenKind::RParen, ")", start),
            '[' => Token::new(TokenKind::LBracket, "[", start),
            ']' => Token::new(TokenKind::RBracket, "]", start),
            '{' => Token::new(TokenKind::LSquirly, "{", start),
            '}' => Token::new(TokenKind::RSquirly, "}", start),
            '<' => Token::new(TokenKind::LAngle, "<", start),
            '>' => Token::new(TokenKind::RAngle, ">", start),
            ':' => Token::new(TokenKind::Colon, ":", start),
            ';' => Token::new(TokenKind::Semicolon, ";", start),
            '.' => Token::new(TokenKind::Dot, ".", start),
            ',' => Token::new(TokenKind::Comma, ",", start),
            '\n' => Token::new(TokenKind::Newline, "\n", start),
            ch if ch.is_ascii_digit() => {
                let text = self.read_while(|ch| ch.is_ascii_digit());
                return Some(Token::new(TokenKind::Int, text, start));
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let text = self
                    .read_while(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == '_');

                let token = if let Some(kind) = KEYWORD_LOOKUP.get(text) {
                    Token::new(*kind, text, start)
                } else {
                    Token::new(TokenKind::Ident, text, start)
                };

                return Some(token);
            }
            _ => return None,
        };

        self.advance();
        Some(token)
    }
}
