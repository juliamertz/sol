use std::borrow::Cow;

use thiserror::Error;

use crate::lexer::memchr::FindByte;

#[derive(Debug, Error)]
pub enum EscapeError {
    #[error("cannot escape: `{0}`")]
    InvalidEscape(char),
    #[error("expected a charachter but none was found")]
    ZeroChars,
}

/// Unescape byte charachter that with leading `\` in a string literal
pub fn unescape_char(ch: char) -> Result<char, EscapeError> {
    Ok(match ch {
        '0' => '\0',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '"' => '"',
        _ => return Err(EscapeError::InvalidEscape(ch)),
    })
}

/// Takes the contents of a string literal (without quotes)
/// and produces a sequence of escaped chars or an error
pub fn unescape_literal<'src>(source: &'src str) -> Result<Cow<'src, str>, EscapeError> {
    if source.as_bytes().find_byte(b'\\').is_none() {
        return Ok(Cow::Borrowed(source));
    }

    let mut buf = Vec::with_capacity(source.len());
    let mut chars = source.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                let next = chars.next().ok_or(EscapeError::ZeroChars)?;
                let escaped = unescape_char(next)?;
                buf.push(escaped)
            }
            other => buf.push(other),
        }
    }

    let escaped = buf.into_iter().collect::<String>();
    Ok(Cow::Owned(escaped))
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    #[test]
    fn unescape_literal() {
        assert_eq!(
            super::unescape_literal("\"\\n\\t\"").unwrap(),
            Cow::Owned::<str>("\"\n\t\"".to_string())
        );
        assert_eq!(
            super::unescape_literal("hello world!").unwrap(),
            Cow::Borrowed("hello world!")
        );
    }
}
