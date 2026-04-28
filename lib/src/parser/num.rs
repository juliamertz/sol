use std::assert_matches;
use std::borrow::Cow;

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

impl Parser<'_> {
    fn num_lit_suffix(&self, suffix: &str) -> Result<LiteralSuffix> {
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

            _ => todo!("invalid number suffix: `{suffix}`"),
        })
    }

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
        let digit_str = clean_digit_str(digit_str);

        let kind = match num.kind {
            NumberKind::Int => {
                let value = i128::from_str_radix(&digit_str, 10)?;
                LiteralKind::Int(value)
            }
            NumberKind::Float { radix_point_idx: _ } => {
                let value = digit_str.parse()?;
                LiteralKind::Float(value)
            }
            NumberKind::Hex => {
                let value = i128::from_str_radix(&digit_str, 16)?;
                LiteralKind::Int(value)
            }
        };

        let suffix = suffix.map(|text| self.num_lit_suffix(text)).transpose()?;

        self.advance()?;

        Ok(Literal {
            id,
            kind,
            span,
            suffix,
        })
    }
}
