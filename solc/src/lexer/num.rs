use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    Int,
    Float { radix_point_idx: usize },
    Hex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadNumber {
    pub len: usize,
    pub prefix_end: Option<usize>,
    pub suffix_start: Option<usize>,
    pub kind: NumberKind,
}

#[derive(Debug, Error)]
pub enum ReadNumberError {
    #[error("unexpected character while reading number: `{0}`")]
    Unexpected(char),
    #[error("floating point number may not have multiple radices")]
    RepeatedRadixPoint,
    #[error("hex number may not have more than one radix prefix")]
    RepeatedRadixPrefix,
}

impl ReadNumber {
    pub fn try_read(source: &str) -> Result<ReadNumber, ReadNumberError> {
        let mut chars = source.char_indices().peekable();
        let mut len = 0;
        let mut kind = None;
        let mut prefix_end = None;
        let mut suffix_start = None;

        while let Some((idx, ch)) = chars.next() {
            match ch {
                ' ' => break,
                '_' => {
                    if let Some(NumberKind::Float { radix_point_idx }) = kind {
                        if radix_point_idx == idx - 1 {
                            return Err(ReadNumberError::Unexpected('_'));
                        }
                    }
                }
                '.' => match kind {
                    Some(NumberKind::Hex) => return Err(ReadNumberError::Unexpected('.')),
                    Some(NumberKind::Float { .. }) => {
                        return Err(ReadNumberError::RepeatedRadixPoint);
                    }
                    _ => {
                        kind = Some(NumberKind::Float {
                            radix_point_idx: idx,
                        })
                    }
                },
                '0' => match chars.peek() {
                    Some((_, 'x')) => match kind {
                        Some(_) => return Err(ReadNumberError::RepeatedRadixPrefix),
                        None => {
                            // consume peeked char
                            let (idx, _) = chars.next().unwrap();
                            len += 1;

                            prefix_end = Some(idx);
                            kind = Some(NumberKind::Hex)
                        }
                    },
                    _ => (),
                },
                ch if ch.is_ascii_digit() => (),
                ch if ch.is_ascii_whitespace() => break,
                ch => {
                    if suffix_start.is_none() {
                        if let Some(NumberKind::Hex) = kind
                            && ch.is_ascii_hexdigit()
                        {
                            // capture hex digits
                        } else {
                            suffix_start = Some(idx);
                        }
                    }
                }
            }

            len += 1;
        }

        let kind = kind.unwrap_or(NumberKind::Int);

        Ok(ReadNumber {
            kind,
            prefix_end,
            suffix_start,
            len,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_int() {
        assert_eq!(
            ReadNumber::try_read("100").unwrap(),
            ReadNumber {
                len: 3,
                prefix_end: None,
                suffix_start: None,
                kind: NumberKind::Int,
            }
        );
        assert_eq!(
            ReadNumber::try_read("100_u32").unwrap(),
            ReadNumber {
                len: 7,
                prefix_end: None,
                suffix_start: Some(4),
                kind: NumberKind::Int,
            }
        );
        assert_eq!(
            ReadNumber::try_read("100_000_000").unwrap(),
            ReadNumber {
                len: 11,
                prefix_end: None,
                suffix_start: None,
                kind: NumberKind::Int,
            }
        );
    }

    #[test]
    fn read_float() {
        assert_eq!(
            ReadNumber::try_read("10.00").unwrap(),
            ReadNumber {
                len: 5,
                prefix_end: None,
                suffix_start: None,
                kind: NumberKind::Float { radix_point_idx: 2 }
            }
        );
        assert_eq!(
            ReadNumber::try_read("100_000.000_f64").unwrap(),
            ReadNumber {
                len: 15,
                prefix_end: None,
                suffix_start: Some(12),
                kind: NumberKind::Float { radix_point_idx: 7 }
            }
        );
    }

    #[test]
    fn read_hex() {
        assert_eq!(
            ReadNumber::try_read("0xFF").unwrap(),
            ReadNumber {
                len: 4,
                prefix_end: Some(1),
                suffix_start: None,
                kind: NumberKind::Hex
            }
        );
        assert_eq!(
            ReadNumber::try_read("0xFF_u32").unwrap(),
            ReadNumber {
                len: 8,
                prefix_end: Some(1),
                suffix_start: Some(5),
                kind: NumberKind::Hex
            }
        );
    }
}
