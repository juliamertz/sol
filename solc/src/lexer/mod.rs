use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::source::{SourceInfo, Span};
use crate::lexer::token::KEYWORD_LOOKUP;

pub mod source;
pub mod token;

pub use crate::lexer::token::{Token, TokenKind};

#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(solc::lexer))]
pub enum LexerError {
    #[error("illegal character: {ch}")]
    Illegal {
        #[source_code]
        src: SourceInfo,
        #[label("here")]
        span: Span,
        ch: char,
    },
    #[error("unterminated string literal")]
    UnterminatedString {
        #[source_code]
        src: SourceInfo,
        #[label("here")]
        span: Span,
    },
}

pub type Result<T> = std::result::Result<T, LexerError>;

#[derive(Debug)]
pub struct Lexer<'src> {
    pub source: SourceInfo,
    pub content: &'src str,
    pub pos: usize,
    pub eof: bool,
}

impl<'src> Lexer<'src> {
    pub fn new(file_path: PathBuf, content: &'src str) -> Self {
        let source = SourceInfo::new(file_path.to_string_lossy(), content.to_string());
        Self {
            source,
            content,
            pos: 0,
            eof: false,
        }
    }

    pub fn source(&self) -> SourceInfo {
        self.source.clone()
    }

    pub fn curr(&self) -> Option<char> {
        self.content
            .as_bytes()
            .get(self.pos)
            .map(|byte| *byte as char)
    }

    pub fn peek(&self) -> Option<char> {
        self.content
            .as_bytes()
            .get(self.pos + 1)
            .map(|byte| *byte as char)
    }

    pub fn advance(&mut self) -> Option<char> {
        self.pos += 1;
        self.curr()
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

    fn read_while<F>(&mut self, condition: F) -> &'src str
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

    fn read_string(&mut self) -> Result<&'src str> {
        let start = self.pos;
        assert_eq!(self.curr(), Some('"'),);
        self.advance();
        let text = self.read_while(|ch| ch != '"');
        if self.curr() != Some('"') {
            Err(LexerError::UnterminatedString {
                src: self.source(),
                span: (start, self.pos - start).into(),
            })
        } else {
            self.advance();
            Ok(text)
        }
    }

    pub fn read_token(&mut self) -> Option<Result<Token<'src>>> {
        self.skip_whitespace();

        // Dirty little hack to return EOF as last token
        if self.curr().is_none() && !self.eof {
            self.eof = true;
            return Some(Ok(Token::new(TokenKind::Eof, "", self.pos)));
        }

        let start = self.pos;

        let token = match self.curr()? {
            '"' => match self.read_string() {
                Ok(text) => {
                    let token = Token::new(TokenKind::String, text, start);
                    return Some(Ok(token));
                }
                Err(err) => return Some(Err(err)),
            },
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
                    return self.read_token();
                } else {
                    Token::new(TokenKind::Sub, "-", start)
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Ne, "!=", start)
                } else {
                    Token::new(TokenKind::Bang, "!", start)
                }
            }
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
                return Some(Ok(Token::new(TokenKind::Int, text, start)));
            }
            ch if ch.is_ascii_alphabetic() || ch == '_' => {
                let text = self
                    .read_while(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == '_');

                let token = if let Some(kind) = KEYWORD_LOOKUP.get(text) {
                    Token::new(*kind, text, start)
                } else {
                    Token::new(TokenKind::Ident, text, start)
                };

                return Some(Ok(token));
            }
            ch => {
                return Some(Err(LexerError::Illegal {
                    src: self.source(),
                    span: (start, 1).into(),
                    ch,
                }));
            }
        };

        self.advance();
        Some(Ok(token))
    }
}
