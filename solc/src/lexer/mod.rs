use std::borrow::Cow;
use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::memchr::FindByte;
use crate::lexer::source::{SourceInfo, Span};
use crate::lexer::token::KEYWORD_LOOKUP;
use crate::lexer::unescape::unescape_literal;

pub mod memchr;
pub mod source;
pub mod token;
pub mod unescape;

#[cfg(test)]
mod test;

pub use crate::lexer::token::{Token, TokenKind};

const ASCII_WHITESPACE_BYTES: [u8; 5] = *b"\t\n\x0C\r ";

#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(solc::lexer))]
pub enum LexerError {
    #[error("illegal character: `{ch}`")]
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
    #[error(transparent)]
    EscapeLiteral(#[from] unescape::EscapeError),
}

pub type Result<T> = std::result::Result<T, LexerError>;

#[derive(Debug)]
pub struct Lexer<'src> {
    source: SourceInfo,
    content: &'src str,
    pos: usize,
    eof: bool,
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

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn reset(&mut self) {
        self.pos = 0;
        self.eof = false;
    }

    pub fn source(&self) -> SourceInfo {
        self.source.clone()
    }

    fn curr(&self) -> Option<u8> {
        self.content.as_bytes().get(self.pos).copied()
    }

    fn peek(&self) -> Option<u8> {
        self.content.as_bytes().get(self.pos + 1).copied()
    }

    fn remaining(&self) -> &[u8] {
        &self.content.as_bytes()[self.pos..self.content.len()]
    }

    fn advance(&mut self) -> Option<u8> {
        self.pos += 1;
        self.curr()
    }

    fn skip_whitespace(&mut self) {
        if let Some(offset) = self.remaining().find_byte_not_in(ASCII_WHITESPACE_BYTES) {
            self.pos += offset;
        } else {
            // if we didn't find any matching byte we can assume we reached eof
            self.eof = true;
        }
    }

    fn read_while<F>(&mut self, condition: F) -> &'src str
    where
        F: Fn(u8) -> bool,
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

    fn read_string(&mut self) -> Result<Cow<'src, str>> {
        let start = self.pos;
        assert_eq!(self.curr(), Some(b'"'),);
        self.advance();
        // TODO: we need to handle escaped quotes here
        let text = self.read_while(|ch| ch != b'"');
        if self.curr() != Some(b'"') {
            Err(LexerError::UnterminatedString {
                src: self.source(),
                span: (start, self.pos - start).into(),
            })
        } else {
            self.advance();
            Ok(unescape_literal(text)?)
        }
    }

    pub fn read_token(&mut self) -> Option<Result<Token<'src>>> {
        self.skip_whitespace();

        if self.curr().is_none() || self.eof {
            return Some(Ok(Token::new(TokenKind::Eof, "", self.pos)));
        }

        let start = self.pos;

        let token = match self.curr()? {
            b'"' => match self.read_string() {
                Ok(text) => {
                    let token = Token::new(TokenKind::String, text, start);
                    return Some(Ok(token));
                }
                Err(err) => return Some(Err(err)),
            },
            b'+' => Token::new(TokenKind::Add, "+", start),
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    Token::new(TokenKind::Eq, "==", start)
                } else {
                    Token::new(TokenKind::Assign, "=", start)
                }
            }
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    Token::new(TokenKind::Arrow, "->", start)
                } else if self.peek() == Some(b'-') {
                    self.read_while(|ch| ch != b'\n');
                    return self.read_token();
                } else {
                    Token::new(TokenKind::Sub, "-", start)
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    Token::new(TokenKind::Ne, "!=", start)
                } else {
                    Token::new(TokenKind::Bang, "!", start)
                }
            }
            b'*' => Token::new(TokenKind::Asterisk, "*", start),
            b'/' => Token::new(TokenKind::Slash, "/", start),
            b'&' => Token::new(TokenKind::Ampersand, "&", start),
            b'(' => Token::new(TokenKind::LParen, "(", start),
            b')' => Token::new(TokenKind::RParen, ")", start),
            b'[' => Token::new(TokenKind::LBracket, "[", start),
            b']' => Token::new(TokenKind::RBracket, "]", start),
            b'{' => Token::new(TokenKind::LSquirly, "{", start),
            b'}' => Token::new(TokenKind::RSquirly, "}", start),
            b'<' => Token::new(TokenKind::LAngle, "<", start),
            b'>' => Token::new(TokenKind::RAngle, ">", start),
            b':' => Token::new(TokenKind::Colon, ":", start),
            b';' => Token::new(TokenKind::Semicolon, ";", start),
            b'.' => Token::new(TokenKind::Dot, ".", start),
            b',' => Token::new(TokenKind::Comma, ",", start),
            b'\n' => Token::new(TokenKind::Newline, "\n", start),
            ch if ch.is_ascii_digit() => {
                let text = self.read_while(|ch| ch.is_ascii_digit());
                return Some(Ok(Token::new(TokenKind::Int, text, start)));
            }
            ch if ch.is_ascii_alphabetic() || ch == b'_' => {
                let text = self
                    .read_while(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == b'_');

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
                    ch: ch as char,
                }));
            }
        };

        self.advance();
        Some(Ok(token))
    }

    pub fn read_until_eof(&mut self) -> Result<Vec<Token<'_>>> {
        let mut buf = vec![];

        while let Some(token) = self.read_token() {
            match token {
                Ok(tok) if tok.kind == TokenKind::Eof => break,
                Ok(tok) => buf.push(tok),
                Err(err) => panic!("failed to read token: {err}"), // TODO: return err variant here
            }
        }

        Ok(buf)
    }
}
