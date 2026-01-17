use std::path::PathBuf;

use crate::lexer::source::SourceInfo;
use crate::lexer::token::{KEYWORD_LOOKUP};

pub mod source;
pub mod token;

pub use crate::lexer::token::{Token, TokenKind};

#[derive(Debug)]
pub struct Lexer {
    pub file_path: PathBuf,
    pub content: String,
    pub pos: usize,
    pub eof: bool,
}

impl Lexer {
    pub fn new(file_path: PathBuf, content: impl ToString) -> Self {
        Self {
            file_path,
            content: content.to_string(),
            pos: 0,
            eof: false,
        }
    }

    pub fn source(&self) -> SourceInfo {
        // TODO: this can be done more effeciently by directly storing sourceinfo in the lexer. for
        // now this will do
        SourceInfo::new(self.file_path.to_string_lossy(), self.content.clone())
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
        self.read_while(|ch| ch != '"')
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
