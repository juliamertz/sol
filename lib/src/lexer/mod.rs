use std::borrow::Cow;
use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::memchr::FindByte;
use crate::lexer::num::ReadNumber;
use crate::lexer::source::{SourceInfo, Span};
use crate::lexer::unescape::unescape_literal;

pub mod memchr;
pub mod num;
pub mod source;
pub mod token;
pub mod unescape;

#[cfg(test)]
mod test;

pub use crate::lexer::token::{Token, TokenKind};

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
    #[error(transparent)]
    ReadNumber(#[from] num::ReadNumberError),
}

pub type Result<T> = std::result::Result<T, LexerError>;

const ASCII_WHITESPACE_BYTES: [u8; 4] = *b"\t\x0C\r ";

/// check if byte is ascii whitespace EXCLUDING the newline character
fn is_ascii_whitespace(byte: &u8) -> bool {
    ASCII_WHITESPACE_BYTES.contains(byte)
}

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

    fn jump_to_eof(&mut self) {
        self.pos = self.content.len() - 1
    }

    fn remaining(&self) -> &'src str {
        &self.content[self.pos..self.content.len()]
    }

    fn remaining_bytes(&self) -> &'src [u8] {
        &self.content.as_bytes()[self.pos..self.content.len()]
    }

    fn advance(&mut self) -> Option<u8> {
        self.pos += 1;
        self.curr()
    }

    fn advance_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn skip_whitespace(&mut self) {
        if let Some(offset) = self
            .remaining_bytes()
            .find_byte_not_in(ASCII_WHITESPACE_BYTES)
        {
            self.pos += offset;
        } else {
            tracing::debug!("did not find non whitespace char, assuming EOF");
            self.jump_to_eof();
        }
    }

    fn skip_comments(&mut self) {
        loop {
            match (self.curr(), self.peek()) {
                (Some(b'-'), Some(b'-')) => {
                    if let Some(end_of_line) = self.remaining_bytes().find_byte(b'\n') {
                        self.pos += end_of_line;
                    } else {
                        tracing::debug!("did not find newline ending comment, assuming EOF");
                        self.jump_to_eof();
                    }
                }
                _ => break,
            }
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
        let start = self.pos();
        assert_eq!(self.curr(), Some(b'"'),);
        self.advance();
        // TODO: we need to handle escaped quotes here
        let text = self.read_while(|ch| ch != b'"');
        if self.curr() != Some(b'"') {
            Err(LexerError::UnterminatedString {
                src: self.source(),
                span: (start, self.pos() - start).into(),
            })
        } else {
            self.advance();
            Ok(unescape_literal(text)?)
        }
    }

    #[inline(always)]
    fn consume_char(&mut self, kind: TokenKind, text: &'static str) -> Token<'src> {
        let token = Token::new(kind, text, self.pos);
        self.advance();
        token
    }

    fn read_token(&mut self) -> Option<Result<Token<'src>>> {
        if self.eof {
            return None;
        }

        let start = self.pos;
        let Some(ch) = self.curr() else {
            self.eof = true;
            return Some(Ok(Token::new(TokenKind::Eof, "", self.pos)));
        };

        let token = match ch {
            b'"' => match self.read_string() {
                Ok(text) => Token::new(TokenKind::String, text, start),
                Err(err) => return Some(Err(err)),
            },
            b'+' => self.consume_char(TokenKind::Add, "+"),
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.advance_n(2);
                    Token::new(TokenKind::Eq, "==", start)
                } else {
                    self.consume_char(TokenKind::Assign, "=")
                }
            }
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.advance_n(2);
                    Token::new(TokenKind::Arrow, "->", start)
                }
                // if we encounter a `--` that means we're reading a comment
                else if self.peek() == Some(b'-') {
                    self.skip_comments();
                    return self.read_token();
                } else {
                    self.consume_char(TokenKind::Sub, "-")
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.advance_n(2);
                    Token::new(TokenKind::Ne, "!=", start)
                } else {
                    self.consume_char(TokenKind::Bang, "!")
                }
            }
            b'*' => self.consume_char(TokenKind::Asterisk, "*"),
            b'/' => self.consume_char(TokenKind::Slash, "/"),
            b'&' => self.consume_char(TokenKind::Ampersand, "&"),
            b'(' => self.consume_char(TokenKind::LParen, "("),
            b')' => self.consume_char(TokenKind::RParen, ")"),
            b'[' => self.consume_char(TokenKind::LBracket, "["),
            b']' => self.consume_char(TokenKind::RBracket, "]"),
            b'{' => self.consume_char(TokenKind::LSquirly, "{"),
            b'}' => self.consume_char(TokenKind::RSquirly, "}"),
            b'<' => self.consume_char(TokenKind::LAngle, "<"),
            b'>' => self.consume_char(TokenKind::RAngle, ">"),
            b':' => self.consume_char(TokenKind::Colon, ":"),
            b';' => self.consume_char(TokenKind::Semicolon, ";"),
            b'.' => self.consume_char(TokenKind::Dot, "."),
            b',' => self.consume_char(TokenKind::Comma, ","),
            b'\n' => self.consume_char(TokenKind::Newline, "\n"),
            ch if ch.is_ascii_digit() => {
                let start = self.pos();
                let remaining = self.remaining();
                let num = match ReadNumber::try_read(remaining) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(LexerError::ReadNumber(err))),
                };
                let text = &remaining[0..num.len];
                self.advance_n(num.len);
                Token::new(TokenKind::Num(num), text, start)
            }
            ch if ch.is_ascii_alphabetic() || ch == b'_' => {
                let text = self
                    .read_while(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == b'_');

                if let Some(kind) = token::lookup_keyword(text) {
                    Token::new(kind, text, start)
                } else {
                    Token::new(TokenKind::Ident, text, start)
                }
            }
            ch if is_ascii_whitespace(&ch) => {
                self.skip_whitespace();
                return self.read_token();
            }
            ch => {
                return Some(Err(LexerError::Illegal {
                    src: self.source(),
                    span: (start, 1).into(),
                    ch: ch as char,
                }));
            }
        };

        tracing::debug!(
            { kind = ?token.kind(), text = token.text.as_ref() },
            "read token"
        );

        Some(Ok(token))
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<Token<'src>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_token()
    }
}
