#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Signedness {
    Signed,
    Unsigned,
}

pub mod encode {
    pub fn bijective_base26(mut n: usize) -> String {
        let mut buf = vec![];
        n += 1;
        while n > 0 {
            n -= 1;
            buf.push(b'a' + (n % 26) as u8);
            n /= 26;
        }
        buf.reverse();

        debug_assert!(buf.iter().all(|ch: &u8| ch.is_ascii_lowercase()));

        // SAFETY: we only ever push bytes to `buf` between 97 and 122
        // since the max output of `n % 26` is 25, and the raw byte value of 'a' is 97 and 97 + 25 = 122
        unsafe { String::from_utf8_unchecked(buf) }
    }
}
