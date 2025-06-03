use std::{collections::HashMap, fmt::Display};

use lazy_static::lazy_static;
use miette::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Int,
    String,
    Ident,

    LParen,
    RParen,

    // Keywords
    Fn,
    Ret,
    If,
    Then,
    Else,
    End,

    // Operators
    Add,
    Sub,
    Arrow,
}

impl TokenKind {
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Fn
                | TokenKind::If
                | TokenKind::Then
                | TokenKind::Else
                | TokenKind::End
                | TokenKind::Ret
        )
    }

    pub fn is_operator(&self) -> bool {
        matches!(self, TokenKind::Add | TokenKind::Sub | TokenKind::Arrow)
    }
}

lazy_static! {
    static ref TOKEN_LOOKUP: HashMap<&'static str, TokenKind> = [
        ("func", TokenKind::Fn),
        ("return", TokenKind::Ret),
        ("if", TokenKind::If),
        ("else", TokenKind::Else),
        ("then", TokenKind::Then),
        ("end", TokenKind::End),
    ]
    .iter()
    .cloned()
    .collect();
}

#[derive(Debug, Clone)]
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
            span: (pos - text.len(), pos).into(),
        }
    }
}

#[derive(Debug)]
pub struct Lexer {
    pub content: String,
    pub pos: usize,
    pub curr: Option<Token>,
    pub next: Option<Token>,
}

// impl<'a> Iterator for Lexer<'a> {
//     type Item = Token;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.pos += 1;

//         // TODO: clones
//         self.curr = self.next.clone();
//         self.next = self.read_token();
//         self.curr.clone()
//     }
// }

impl Lexer {
    pub fn new(content: impl ToString) -> Self {
        Self {
            content: content.to_string(),
            pos: 0,
            curr: None,
            next: None, // TODO:
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

    fn read_until(&mut self, until: char) -> &str {
        self.read_while(|ch| ch != until)
    }

    fn read_string(&mut self) -> &str {
        assert_eq!(self.curr(), Some('"'),);

        self.advance();
        self.read_until('"')
    }

    pub fn read_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let token = match self.curr()? {
            '"' => Token::new(TokenKind::String, self.read_string().to_string(), self.pos),
            '+' => Token::new(TokenKind::Add, "+", self.pos),
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token::new(TokenKind::Arrow, "->", self.pos)
                } else {
                    Token::new(TokenKind::Sub, "-", self.pos)
                }
            }
            '(' => Token::new(TokenKind::LParen, "(", self.pos),
            ')' => Token::new(TokenKind::RParen, ")", self.pos),
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

fn is_whitespace(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}
