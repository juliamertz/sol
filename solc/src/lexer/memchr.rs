use std::simd::cmp::SimdPartialEq;
use std::simd::{Mask, Simd};

const CHUNK_SIZE: usize = 16;

/// Inverse implementation of `memchr` over a generic `N` of needles
#[inline(always)]
fn memchrn_inv<const N: usize>(needles: [u8; N], haystack: &[u8]) -> Option<usize> {
    let splats = needles.map(|needle| Simd::<u8, CHUNK_SIZE>::splat(needle));
    let chunks = haystack.chunks_exact(CHUNK_SIZE);
    let remainder = chunks.remainder();
    let chunked = chunks
        .enumerate()
        .map(|(idx, chunk)| (idx * CHUNK_SIZE, chunk));

    for (offset, chunk) in chunked {
        let haystack = Simd::<u8, CHUNK_SIZE>::from_slice(chunk);
        let mut mask = Mask::splat(true);
        for needle in &splats {
            mask &= haystack.simd_ne(*needle);
        }
        let bits = mask.to_bitmask();
        if bits != 0 {
            let idx = offset + bits.trailing_zeros() as usize;
            return Some(idx);
        }
    }

    if !remainder.is_empty() {
        return remainder
            .iter()
            .position(|byte| !needles.contains(byte))
            .map(|offset| offset + haystack.len() - remainder.len());
    }

    None
}

pub trait FindByte {
    /// Search for the first occurence of a byte that does not match one of the `N` input bytes
    fn find_byte_not_in<const N: usize>(&self, needles: [u8; N]) -> Option<usize>;
}

impl FindByte for &[u8] {
    fn find_byte_not_in<const N: usize>(&self, needles: [u8; N]) -> Option<usize> {
        memchrn_inv(needles, self)
    }
}

#[cfg(test)]
mod test {
    extern crate test;

    use test::Bencher;

    /// Scalar reference implementation for any N.
    fn naive<const N: usize>(needles: [u8; N], haystack: &[u8]) -> Option<usize> {
        haystack.iter().position(|b| !needles.contains(b))
    }

    /// Assert SIMD result matches the naive scalar result.
    fn assert_eq_naive<const N: usize>(needles: [u8; N], haystack: &[u8]) {
        let expected = naive(needles, haystack);
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
        assert_eq_naive([b' '], b"");
        assert_eq_naive([b'a', b'b'], b"");
    }

    #[test]
    fn all_needles() {
        // Every byte is a needle → no match.
        assert_eq_naive([b' '], b"          ");
        assert_eq_naive([b'a', b'b'], b"ababababab");
        // Exactly one chunk of needles.
        assert_eq_naive([b'x'], &[b'x'; super::CHUNK_SIZE]);
    }

    #[test]
    fn first_byte_differs() {
        assert_eq_naive([b' '], b"x         ");
        assert_eq_naive([b'a', b'b'], b"z");
    }

    #[test]
    fn last_byte_differs() {
        assert_eq_naive([b' '], b"         x");
        let mut buf = vec![b'a'; 100];
        *buf.last_mut().unwrap() = b'z';
        assert_eq_naive([b'a'], &buf);
    }

    #[test]
    fn hit_in_remainder() {
        // Length not a multiple of CHUNK_SIZE; match falls in the tail.
        let mut buf = vec![b'.'; super::CHUNK_SIZE + 3];
        *buf.last_mut().unwrap() = b'!';
        assert_eq_naive([b'.'], &buf);
    }

    #[test]
    fn hit_at_chunk_boundary() {
        // Match is the first byte of the second chunk.
        let mut buf = vec![b' '; super::CHUNK_SIZE * 2];
        buf[super::CHUNK_SIZE] = b'x';
        assert_eq_naive([b' '], &buf);
    }

    #[test]
    fn single_needle() {
        assert_eq_naive([b'\n'], b"\n\n\n\nhello");
    }

    #[test]
    fn many_needles() {
        let ws: [u8; 5] = [b'\t', b'\n', b'\x0C', b'\r', b' '];
        let hay = " ".repeat(4025) + &"\n".repeat(5075) + "l";
        assert_eq_naive(ws, hay.as_bytes());
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
        b.iter(|| naive([b' ', b'\n'], hay.as_bytes()));
    }

    #[bench]
    fn bench_simd(b: &mut Bencher) {
        let hay = test::black_box(bench_haystack());
        b.iter(|| super::memchrn_inv([b' ', b'\n'], hay.as_bytes()));
    }
}
