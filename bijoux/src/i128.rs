//! Bijective variable-length encoding for signed 128-bit integers.
//!
//! bijou128s composes the standard [zigzag] bijection with the
//! [`bijou128`](crate::u128) wire format:
//!
//! ```text
//! encode(n) = bijou128_encode(zigzag(n))     zigzag(n) = (n << 1) ^ (n >> 127)
//! ```
//!
//! Small-magnitude values of either sign get short encodings; all
//! structural properties (canonicality, length from the first byte,
//! 1–17 byte range) are inherited from bijou128.
//!
//! See the [specification](https://github.com/inkandswitch/bijou/blob/main/bijoux/specs/bijou128s.md)
//! for the full format definition and test vectors.
//!
//! # Encoding
//!
//! Single-byte window: `[-120, +119]` (bijou128's 240 single-byte
//! codes, interleaved around zero).
//!
//! # Ordering caveat
//!
//! Byte-lexicographic order is **zigzag order, not numeric order**
//! (`0, -1, 1, -2, 2, …`). See [`crate::i64`]'s module docs for the
//! full discussion; it applies identically here.
//!
//! # Examples
//!
//! ```
//! let mut buf = Vec::new();
//! bijoux::i128::encode(-1, &mut buf);
//! assert_eq!(buf, [0x01]);
//!
//! let (value, len) = bijoux::i128::decode(&buf).unwrap();
//! assert_eq!((value, len), (-1, 1));
//! ```
//!
//! [zigzag]: https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding

pub use crate::u128::{DecodeError, MAX_BYTES};

use alloc::vec::Vec;

/// Stack-only encoded form of a bijou128s value.
///
/// A distinct type from [`crate::u128::EncodedU128`] even though the byte
/// container is identical (a bijou128s encoding *is* a u128-format encoding
/// of the zigzag-mapped value): keeping the types separate makes
/// signed/unsigned mix-ups — e.g. inserting a signed encoding into a
/// map keyed by unsigned ones — a compile error instead of a silent
/// logic bug.
///
/// `Ord`/`PartialOrd` compare byte-lexicographically, which for this
/// signed format is **zigzag order, not numeric order** (see the
/// module docs). `Eq`/`Hash`/`Borrow<[u8]>` are byte-wise and
/// mutually consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
// Same container, signed interpretation: bijou128s is defined as
// zigzag ∘ bijou128, so this wraps the unsigned encoder's output and
// adds only the type-level domain distinction (see module docs).
pub struct EncodedI128(crate::u128::EncodedU128);

impl EncodedI128 {
    /// Length of the encoding in bytes (always in `1..=MAX_BYTES`).
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`; bijou encodings are never empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The encoded bytes as a `&[u8]` of the correct length.
    #[inline]
    #[must_use]
    pub const fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl core::ops::Deref for EncodedI128 {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl AsRef<[u8]> for EncodedI128 {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl core::borrow::Borrow<[u8]> for EncodedI128 {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl IntoIterator for EncodedI128 {
    type Item = u8;
    type IntoIter = <crate::u128::EncodedU128 as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a EncodedI128 {
    type Item = &'a u8;
    type IntoIter = <&'a crate::u128::EncodedU128 as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

/// Map an `i128` onto a `u128` by interleaving around zero:
/// `0, -1, 1, -2, 2, … → 0, 1, 2, 3, 4, …`.
#[inline]
#[must_use]
#[allow(clippy::cast_sign_loss)] // reinterpreting the bits is the point
pub const fn zigzag(n: i128) -> u128 {
    ((n << 1) ^ (n >> 127)) as u128
}

/// Inverse of [`zigzag`].
#[inline]
#[must_use]
#[allow(clippy::cast_possible_wrap)] // the wrap is the point
pub const fn unzigzag(u: u128) -> i128 {
    ((u >> 1) as i128) ^ -((u & 1) as i128)
}

/// Returns the number of bytes needed to encode `value` (1..=17).
///
/// # Examples
///
/// ```
/// assert_eq!(bijoux::i128::encoded_len(0), 1);
/// assert_eq!(bijoux::i128::encoded_len(-120), 1);
/// assert_eq!(bijoux::i128::encoded_len(119), 1);
/// assert_eq!(bijoux::i128::encoded_len(120), 2);
/// assert_eq!(bijoux::i128::encoded_len(i128::MIN), 17);
/// ```
#[inline]
#[must_use]
pub const fn encoded_len(value: i128) -> usize {
    crate::u128::encoded_len(zigzag(value))
}

/// Encodes `value`, appending the bytes to `buf`.
#[inline]
pub fn encode(value: i128, buf: &mut Vec<u8>) {
    crate::u128::encode(zigzag(value), buf);
}

/// Encodes `value` without allocating, returning an [`EncodedI128`].
#[inline]
#[must_use]
pub const fn encoded_bytes(value: i128) -> EncodedI128 {
    EncodedI128(crate::u128::encoded_bytes(zigzag(value)))
}

/// Decodes one value from the front of `buf`, returning it along with
/// the number of bytes consumed.
///
/// # Errors
///
/// Returns [`DecodeError::BufferTooShort`] on truncated input, or
/// [`DecodeError::Overflow`] if a 17-byte payload exceeds the `u128`
/// (zigzag) range.
#[inline]
pub const fn decode(buf: &[u8]) -> Result<(i128, usize), DecodeError> {
    match crate::u128::decode(buf) {
        Ok((value, consumed)) => Ok((unzigzag(value), consumed)),
        Err(err) => Err(err),
    }
}

/// Returns a lazy iterator decoding every value in `buf`; fuses after
/// the first error.
#[must_use]
pub const fn decode_iter(buf: &[u8]) -> DecodeIter<'_> {
    DecodeIter {
        inner: crate::u128::decode_iter(buf),
    }
}

/// Lazy decoding iterator returned by [`decode_iter`].
#[derive(Debug)]
pub struct DecodeIter<'a> {
    inner: crate::u128::DecodeIter<'a>,
}

impl Iterator for DecodeIter<'_> {
    type Item = Result<i128, DecodeError>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(Ok(value)) => Some(Ok(unzigzag(value))),
            Some(Err(err)) => Some(Err(err)),
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl core::iter::FusedIterator for DecodeIter<'_> {}

/// Decodes every value in `buf` as a `Vec<i128>` (all-or-nothing).
///
/// # Errors
///
/// Returns the first [`DecodeError`] encountered.
pub fn decode_all(buf: &[u8]) -> Result<Vec<i128>, DecodeError> {
    decode_iter(buf).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = core::result::Result<(), DecodeError>;

    const VECTORS: &[(i128, &[u8])] = &[
        (0, &[0x00]),
        (-1, &[0x01]),
        (1, &[0x02]),
        (119, &[0xEE]),        // zigzag = 238
        (-120, &[0xEF]),       // zigzag = 239, last 1-byte
        (120, &[0xF0, 0x00]),  // zigzag = 240, first 2-byte
        (-121, &[0xF0, 0x01]), // zigzag = 241
        (247, &[0xF0, 0xFE]),
        (-248, &[0xF0, 0xFF]),
        (248, &[0xF1, 0x00, 0x00]),
        (
            i128::MAX,
            &[
                0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
                0xFE, 0xFE, 0x0E,
            ],
        ),
        (
            i128::MIN,
            &[
                0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
                0xFE, 0xFE, 0x0F,
            ],
        ),
    ];

    #[test]
    fn spec_vectors_round_trip() -> TestResult {
        for &(value, expected) in VECTORS {
            let mut buf = Vec::new();
            encode(value, &mut buf);
            assert_eq!(buf, expected, "encode({value})");
            assert_eq!(encoded_len(value), expected.len());
            assert_eq!(decode(expected)?, (value, expected.len()));
        }

        Ok(())
    }

    #[test]
    fn definitional_equivalence_and_extremes() -> TestResult {
        for value in [
            0i128,
            -1,
            1,
            -120,
            119,
            -121,
            120,
            i128::from(i64::MIN),
            i128::from(i64::MAX),
            i128::MIN,
            i128::MAX,
            i128::MIN + 1,
            i128::MAX - 1,
        ] {
            let z = zigzag(value);
            assert_eq!(unzigzag(z), value);
            assert_eq!(
                encoded_bytes(value).as_slice(),
                crate::u128::encoded_bytes(z).as_slice()
            );
            let (decoded, consumed) = decode(encoded_bytes(value).as_slice())?;
            assert_eq!(decoded, value);
            assert_eq!(consumed, encoded_len(value));
        }
        assert_eq!(zigzag(i128::MIN), u128::MAX);
        assert_eq!(encoded_len(i128::MIN), MAX_BYTES);
        assert_eq!(encoded_len(i128::MAX), MAX_BYTES);

        Ok(())
    }

    /// Both signs on both sides of every tier edge — all 16 tiers
    /// (see `i64.rs`'s twin for the derivation).
    #[test]
    fn tier_edges_both_signs() -> TestResult {
        const THRESHOLD: u128 = 240;
        const TIERS: u32 = 16;

        let mut offset: u128 = THRESHOLD;
        for n in 1..=TIERS {
            let below = n as usize;
            let at = n as usize + 1;
            for (z, expected_len) in [
                (offset - 2, below),
                (offset - 1, below),
                (offset, at),
                (offset + 1, at),
            ] {
                let value = unzigzag(z);
                assert_eq!(encoded_len(value), expected_len, "z={z} value={value}");
                let mut buf = Vec::new();
                encode(value, &mut buf);
                assert_eq!(buf.len(), expected_len);
                assert_eq!(decode(&buf)?, (value, expected_len));
            }
            if n < TIERS {
                offset += 1u128 << (8 * n);
            }
        }

        assert_eq!(encoded_len(unzigzag(u128::MAX)), MAX_BYTES);
        assert_eq!(encoded_len(unzigzag(u128::MAX - 1)), MAX_BYTES);

        Ok(())
    }

    #[test]
    fn errors_propagate() {
        assert_eq!(decode(&[]), Err(DecodeError::BufferTooShort));
        assert_eq!(decode(&[0xF0]), Err(DecodeError::BufferTooShort));
        assert_eq!(decode(&[0xFF; 17]), Err(DecodeError::Overflow));
    }

    #[test]
    fn decode_iter_and_all() -> TestResult {
        let mut buf = Vec::new();
        for v in [-2i128, -1, 0, 1, 2, i128::MIN, i128::MAX] {
            encode(v, &mut buf);
        }
        assert_eq!(decode_all(&buf)?, [-2, -1, 0, 1, 2, i128::MIN, i128::MAX]);

        Ok(())
    }

    #[cfg(feature = "bolero")]
    #[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::panic)]
    mod property {
        use super::*;

        #[test]
        #[cfg_attr(miri, ignore)]
        fn round_trip() {
            bolero::check!()
                .with_arbitrary::<i128>()
                .for_each(|&value| {
                    let mut buf = Vec::new();
                    encode(value, &mut buf);
                    let (decoded, consumed) = decode(&buf).unwrap_or_else(|e| {
                        panic!("round-trip decode failed for {value}: {e}");
                    });
                    assert_eq!(decoded, value);
                    assert_eq!(consumed, buf.len());
                    assert_eq!(encoded_len(value), buf.len());
                    assert_eq!(encoded_bytes(value).as_slice(), &buf[..]);
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn canonical_all_decodable_buffers_reencode_to_themselves() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|bytes| {
                    if let Ok((value, consumed)) = decode(bytes) {
                        let mut re = Vec::new();
                        encode(value, &mut re);
                        assert_eq!(&bytes[..consumed], &re[..], "non-canonical decode");
                    }
                });
        }
    }
}
