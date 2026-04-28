use std::assert_matches;
use std::borrow::Cow;

use thiserror::Error;

use crate::ast::{FloatTy, IntTy, Literal, LiteralKind, LiteralSuffix, UIntTy};
use crate::lexer::num::{NumberKind, ReadNumber};
use crate::lexer::token::TokenKind;

use super::{Parser, Result};

fn clean_digit_str<'a>(text: &'a str) -> Cow<'a, str> {
    if !text.contains('_') {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.chars().filter(|ch| *ch != '_').collect())
    }
}

fn split_digit_str<'a>(
    text: &'a str,
    num: &ReadNumber,
) -> (Option<&'a str>, &'a str, Option<&'a str>) {
    let mut buf = text;

    let prefix = num.prefix_end.map(|idx| {
        buf = &buf[idx + 1..];
        &text[..=idx]
    });

    let suffix = num.suffix_start.map(|idx| {
        let prefix_len = prefix.map(|str| str.len()).unwrap_or(0);
        buf = &buf[..idx - prefix_len];
        &text[idx..]
    });

    (prefix, buf, suffix)
}

fn digit_from_byte<T: From<u8>>(byte: u8) -> T {
    T::from(byte - b'0')
}

#[derive(Debug, Error)]
pub enum ParseSuffixError {
    #[error("invalid literal suffix: `{0}`")]
    Invalid(String),
}

fn parse_suffix(suffix: &str) -> Result<LiteralSuffix, ParseSuffixError> {
    use LiteralSuffix::*;
    Ok(match suffix {
        "i8" => Int(IntTy::I8),
        "i16" => Int(IntTy::I16),
        "i32" => Int(IntTy::I32),
        "i64" => Int(IntTy::I64),
        "u8" => UInt(UIntTy::U8),
        "u16" => UInt(UIntTy::U16),
        "u32" => UInt(UIntTy::U32),
        "u64" => UInt(UIntTy::U64),
        "f16" => Float(FloatTy::F16),
        "f32" => Float(FloatTy::F32),
        "f64" => Float(FloatTy::F64),
        _ => return Err(ParseSuffixError::Invalid(suffix.into())),
    })
}

/// Parse a valid floating point number from string
fn parse_float_unchecked(text: &str, radix_point_idx: usize) -> f64 {
    let (integer_str, rhs) = text.split_at(radix_point_idx);
    let fractional_str = &rhs[1..];

    let mut result = 0.0;
    let mut multiplier = 1.0;

    for byte in integer_str.bytes().rev() {
        if byte == b'_' {
            continue;
        }
        let digit = digit_from_byte::<f64>(byte);
        result += digit * multiplier;
        multiplier *= 10.0;
    }

    let mut divider = 10.0;

    for byte in fractional_str.bytes() {
        if byte == b'_' {
            continue;
        }
        let digit = digit_from_byte::<f64>(byte);
        result += digit / divider;
        divider *= 10.0;
    }

    result
}

#[derive(Debug, Error)]
pub enum ParseNumberError {
    #[error(transparent)]
    Suffix(#[from] ParseSuffixError),
    #[error("failed to parse integer: {0}")]
    Int(#[from] std::num::ParseIntError),
}

impl Parser<'_> {
    pub(super) fn num_lit(&mut self, num: ReadNumber) -> Result<Literal> {
        assert_matches!(
            self.curr.kind,
            TokenKind::Num(_),
            "`self.curr` must be of kind `TokenKind::Num(_)`"
        );

        let id = self.ctx.next_id();
        let span = self.curr.span();
        let text = self.curr.text.as_ref();
        let (_, digit_str, suffix) = split_digit_str(&text, &num);

        let kind = match num.kind {
            NumberKind::Int => {
                let digit_str = clean_digit_str(text);
                let value = i128::from_str_radix(&digit_str, 10).map_err(ParseNumberError::Int)?;
                LiteralKind::Int(value)
            }
            NumberKind::Float { radix_point_idx } => {
                let value = parse_float_unchecked(&digit_str, radix_point_idx);
                LiteralKind::Float(value)
            }
            NumberKind::Hex => {
                let digit_str = clean_digit_str(text);
                let value = i128::from_str_radix(&digit_str, 16).map_err(ParseNumberError::Int)?;
                LiteralKind::Int(value)
            }
        };

        let suffix = suffix
            .map(parse_suffix)
            .transpose()
            .map_err(ParseNumberError::Suffix)?;

        self.advance()?;

        Ok(Literal {
            id,
            kind,
            span,
            suffix,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_float() {
        assert_matches!(
            parse_float_unchecked("123.4", 3),
            123.4,
            "parse basic float without radix point"
        );
        assert_matches!(
            parse_float_unchecked("01234.0", 5),
            1234.0,
            "parse float with leading 0"
        );
        assert_matches!(
            parse_float_unchecked("100_000.5", 7),
            100_000.5,
            "parse float with underscores"
        );
        assert_matches!(
            parse_float_unchecked("1234.56", 4),
            1234.56,
            "parse float with radix point"
        );
    }
}
