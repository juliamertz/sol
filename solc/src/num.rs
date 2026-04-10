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
        String::from_utf8(buf).unwrap()
    }
}
