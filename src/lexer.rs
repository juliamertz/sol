use std::{collections::HashMap, fmt::Display};

use lazy_static::lazy_static;
use miette::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Eof,

    // Literals
    Int,
    String,
    Ident,

    LParen,
    RParen,
    LBracket,
    RBracket,
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

    // Operators
    Eq,
    Assign,
    Add,
    Sub,
    Asterisk,
    Slash,
    Lt,
    Gt,
    Arrow,
    And,
    Or,
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
                | TokenKind::Then
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
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::And
                | TokenKind::Or
        )
    }
}

lazy_static! {
    static ref TOKEN_LOOKUP: HashMap<&'static str, TokenKind> = [
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
    ]
    .iter()
    .cloned()
    .collect();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: SourceSpan,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Token {
    pub fn new(kind: TokenKind, text: impl ToString, pos: usize) -> Self {
        let text = text.to_string();
        Self {
            kind,
            text: text.to_string(),
            span: (pos, text.len()).into(),
        }
    }
}

#[derive(Debug)]
pub struct Lexer {
    pub content: String,
    pub pos: usize,
    pub eof: bool, // TODO: i don't like this hack
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
            if ch.is_ascii_whitespace() {
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

        let token = match self.curr()? {
            '"' => Token::new(TokenKind::String, self.read_string().to_string(), self.pos),
            '+' => Token::new(TokenKind::Add, "+", self.pos),
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Eq, "==", self.pos)
                } else {
                    Token::new(TokenKind::Assign, "=", self.pos)
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token::new(TokenKind::Arrow, "->", self.pos)
                } else if self.peek() == Some('-') {
                    self.read_while(|ch| ch != '\n');
                    self.read_token()?
                } else {
                    Token::new(TokenKind::Sub, "-", self.pos)
                }
            }
            '*' => Token::new(TokenKind::Asterisk, "*", self.pos),
            '/' => Token::new(TokenKind::Slash, "/", self.pos),
            '<' => Token::new(TokenKind::Lt, "<", self.pos),
            '>' => Token::new(TokenKind::Gt, ">", self.pos),
            '(' => Token::new(TokenKind::LParen, "(", self.pos),
            ')' => Token::new(TokenKind::RParen, ")", self.pos),
            '[' => Token::new(TokenKind::LBracket, "[", self.pos),
            ']' => Token::new(TokenKind::RBracket, "]", self.pos),
            ':' => Token::new(TokenKind::Colon, ":", self.pos),
            ';' => Token::new(TokenKind::Semicolon, ";", self.pos),
            ',' => Token::new(TokenKind::Comma, ",", self.pos),
            ch if ch.is_ascii_digit() => {
                let start = self.pos;
                let text = self.read_while(|ch| ch.is_ascii_digit());
                return Some(Token::new(TokenKind::Int, text, start + text.len()));
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let start = self.pos;
                let text = self.read_while(|ch| ch.is_ascii_alphabetic() || ch == '_');

                let token = if let Some(kind) = TOKEN_LOOKUP.get(text) {
                    Token::new(*kind, text, text.len() + start)
                } else {
                    Token::new(TokenKind::Ident, text, text.len() + start)
                };

                return Some(token);
            }
            _ => return None,
        };

        self.advance();
        Some(token)
    }
}
