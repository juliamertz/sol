use std::simd::cmp::SimdPartialEq;
use std::simd::{Mask, u8x16};

const CHUNK_SIZE: usize = 16;

#[inline(always)]
fn simd_find_chunked<const N: usize, F>(needles: [u8; N], haystack: &[u8], callback: F) -> Option<usize>
where
    F: Fn([u8x16; N], u8x16) -> Option<usize>,
{
    let splats = needles.map(u8x16::splat);
    let chunks = haystack.chunks_exact(CHUNK_SIZE);
    let remainder = chunks.remainder();

    for (offset, chunk) in chunks
        .enumerate()
        .map(|(idx, chunk)| (CHUNK_SIZE * idx, chunk))
    {
        let haystack = u8x16::from_slice(chunk);
        if let Some(idx) = callback(splats, haystack) {
            return Some(offset + idx);
        };
    }

    if !remainder.is_empty() {
        return callback(splats, u8x16::load_or_default(remainder))
            .map(|idx| idx + haystack.len() - remainder.len())
            .filter(|idx| *idx < haystack.len());
    }

    None
}

#[inline(always)]
fn memchrn<const N: usize>(needles: [u8; N], haystack: &[u8]) -> Option<usize> {
    simd_find_chunked(needles, haystack, |needles, haystack| {
        let mut mask = Mask::splat(false);
        for needle in &needles {
            mask |= haystack.simd_eq(*needle);
        }
        let bits = mask.to_bitmask();
        if bits != 0 {
            Some(bits.trailing_zeros() as usize)
        } else {
            None
        }
    })
}

/// Inverse implementation of `memchr` over a generic `N` of needles
#[inline(always)]
fn memchrn_inv<const N: usize>(needles: [u8; N], haystack: &[u8]) -> Option<usize> {
    simd_find_chunked(needles, haystack, |needles, haystack| {
        let mut mask = Mask::splat(true);
        for needle in &needles {
            mask &= haystack.simd_ne(*needle);
        }
        let bits = mask.to_bitmask();
        if bits != 0 {
            Some(bits.trailing_zeros() as usize)
        } else {
            None
        }
    })
}

pub trait FindByte {
    /// Search for the first occurence of a byte that does not match one of the `N` input bytes
    fn find_byte_in<const N: usize>(&self, needles: [u8; N]) -> Option<usize>;

    fn find_byte_not_in<const N: usize>(&self, needles: [u8; N]) -> Option<usize>;

    fn find_byte(&self, needle: u8) -> Option<usize> {
        self.find_byte_in([needle])
    }
}

impl FindByte for &[u8] {
    fn find_byte_in<const N: usize>(&self, needles: [u8; N]) -> Option<usize> {
        memchrn(needles, self)
    }

    fn find_byte_not_in<const N: usize>(&self, needles: [u8; N]) -> Option<usize> {
        memchrn_inv(needles, self)
    }
}

#[cfg(test)]
mod test {
    extern crate test;

    use test::Bencher;

    /// Scalar inverse reference implementation for any N.
    fn naive_inv<const N: usize>(needles: [u8; N], haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|b| !needles.contains(b))
    }

    /// Assert SIMD result matches the naive scalar result.
    fn assert_eq_naive_inv<const N: usize>(needles: [u8; N], haystack: &[u8]) {
        let expected = naive_inv(needles, haystack);
        let actual = super::memchrn_inv(needles, haystack);
        assert_eq!(
            actual,
            expected,
            "needles={needles:?} haystack len={}",
            haystack.len()
        );
    }

    #[test]
    fn empty_haystack() {
        assert_eq_naive_inv([b' '], b"");
        assert_eq_naive_inv([b'a', b'b'], b"");
    }

    #[test]
    fn all_needles() {
        // Every byte is a needle → no match.
        assert_eq_naive_inv([b' '], b"          ");
        assert_eq_naive_inv([b'a', b'b'], b"ababababab");
        // Exactly one chunk of needles.
        assert_eq_naive_inv([b'x'], &[b'x'; super::CHUNK_SIZE]);
    }

    #[test]
    fn first_byte_differs() {
        assert_eq_naive_inv([b' '], b"x         ");
        assert_eq_naive_inv([b'a', b'b'], b"z");
    }

    #[test]
    fn last_byte_differs() {
        assert_eq_naive_inv([b' '], b"         x");
        let mut buf = vec![b'a'; 100];
        *buf.last_mut().unwrap() = b'z';
        assert_eq_naive_inv([b'a'], &buf);
    }

    #[test]
    fn hit_in_remainder() {
        // Length not a multiple of CHUNK_SIZE; match falls in the tail.
        let mut buf = vec![b'.'; super::CHUNK_SIZE + 3];
        *buf.last_mut().unwrap() = b'!';
        assert_eq_naive_inv([b'.'], &buf);
    }

    #[test]
    fn hit_at_chunk_boundary() {
        // Match is the first byte of the second chunk.
        let mut buf = vec![b' '; super::CHUNK_SIZE * 2];
        buf[super::CHUNK_SIZE] = b'x';
        assert_eq_naive_inv([b' '], &buf);
    }

    #[test]
    fn single_needle() {
        assert_eq_naive_inv([b'\n'], b"\n\n\n\nhello");
    }

    #[test]
    fn many_needles() {
        let ws: [u8; 5] = [b'\t', b'\n', b'\x0C', b'\r', b' '];
        let hay = " ".repeat(4025) + &"\n".repeat(5075) + "l";
        assert_eq_naive_inv(ws, hay.as_bytes());
    }

    #[test]
    fn all_256_byte_values() {
        // Build a haystack of bytes 0..=254, needle = every byte except 42.
        // The first non-needle byte should be at index 42.
        let haystack: Vec<u8> = (0..=254).collect();
        let result = super::memchrn_inv([42], &haystack);
        assert_eq!(result, Some(0)); // 0 != 42

        // All-needle haystack: fill with 42s.
        let all42 = vec![42u8; 300];
        assert_eq!(super::memchrn_inv([42], &all42), None);
    }

    #[test]
    fn trait_impl() {
        use super::FindByte;
        let data: &[u8] = b"   hello";
        assert_eq!(data.find_byte_not_in([b' ']), Some(3));
    }

    // Benchmarks

    fn bench_haystack() -> String {
        " ".repeat(4025)
            + &"\n".repeat(5075)
            + "l"
            + &" ".repeat(5)
            + "l"
            + &" ".repeat(super::CHUNK_SIZE)
    }

    #[bench]
    fn bench_naive(b: &mut Bencher) {
        let hay = test::black_box(bench_haystack());
        b.iter(|| naive_inv([b' ', b'\n'], hay.as_bytes()));
    }

    #[bench]
    fn bench_simd(b: &mut Bencher) {
        let hay = test::black_box(bench_haystack());
        b.iter(|| super::memchrn_inv([b' ', b'\n'], hay.as_bytes()));
    }
}
