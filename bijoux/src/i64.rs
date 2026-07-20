//! Bijective variable-length encoding for signed 64-bit integers.
//!
//! bijou64s (**BIJ**ective **O**ffset **U64**, **s**igned) encodes `i64`
//! values into 1–9 bytes by composing the standard [zigzag] bijection
//! with the [`bijou64`](crate::u64) wire format:
//!
//! ```text
//! encode(n) = bijou64_encode(zigzag(n))      zigzag(n) = (n << 1) ^ (n >> 63)
//! ```
//!
//! Zigzag interleaves the signed integers around zero (`0, -1, 1, -2,
//! 2, …` → `0, 1, 2, 3, 4, …`), so small-magnitude values of **either
//! sign** get short encodings. Everything structural is inherited from
//! bijou64: one byte for the 248 smallest mapped values, length
//! determined by the first byte alone, structural canonicality (each
//! `i64` has exactly one encoding and vice versa).
//!
//! See the [specification](https://github.com/inkandswitch/bijou/blob/main/bijoux/specs/bijou64s.md)
//! for the full format definition and test vectors.
//!
//! # Encoding
//!
//! Single-byte window: `[-124, +123]`. Tier boundaries in signed terms:
//!
//! ```text
//! ┌───────┬─────────────────────────────────────────────┐
//! │ Bytes │ Signed range                                │
//! ├───────┼─────────────────────────────────────────────┤
//! │ 1     │ -124 ..= 123                                │
//! │ 2     │ -252 ..= -125  and  124 ..= 251             │
//! │ 3     │ -33_020 ..= -253  and  252 ..= 33_019       │
//! │ ⋮     │ (each bijou64 tier, split around zero)      │
//! │ 9     │ down to i64::MIN and up to i64::MAX         │
//! └───────┴─────────────────────────────────────────────┘
//! ```
//!
//! # Ordering caveat
//!
//! Unlike the unsigned formats, **byte-lexicographic order is _not_
//! numeric order** for bijou64s — it is zigzag order (`0, -1, 1, -2,
//! 2, …`, i.e. by magnitude, negatives first within equal magnitude).
//! [`EncodedI64`]' `Ord` therefore compares in zigzag order too. If
//! you need memcomparable signed keys, this is not the format for you.
//!
//! # Examples
//!
//! ```
//! let mut buf = Vec::new();
//! bijoux::i64::encode(-1, &mut buf);
//! assert_eq!(buf, [0x01]); // zigzag(-1) = 1
//!
//! let (value, len) = bijoux::i64::decode(&buf).unwrap();
//! assert_eq!((value, len), (-1, 1));
//! ```
//!
//! [zigzag]: https://protobuf.dev/programming-guides/encoding/#signed-ints

pub use crate::u64::{DecodeError, MAX_BYTES};

/// Stack-only encoded form of a bijou64s value.
///
/// The byte container is identical to the unsigned format's — a
/// bijou64s encoding *is* a bijou64 encoding of the zigzag-mapped
/// value — so this is an alias of [`crate::u64::EncodedU64`],
/// not a distinct type. Note the inherited `Ord` compares
/// byte-lexicographically, which for this signed format is **zigzag
/// order, not numeric order** (see the module docs).
pub type EncodedI64 = crate::u64::EncodedU64;

use alloc::vec::Vec;

/// Map an `i64` onto a `u64` by interleaving around zero:
/// `0, -1, 1, -2, 2, … → 0, 1, 2, 3, 4, …`.
///
/// Branchless; the arithmetic right shift smears the sign bit into a
/// mask that flips all bits of `n << 1` exactly when `n` is negative.
#[inline]
#[must_use]
#[allow(clippy::cast_sign_loss)] // reinterpreting the bits is the point
pub const fn zigzag(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)) as u64
}

/// Inverse of [`zigzag`].
#[inline]
#[must_use]
#[allow(clippy::cast_possible_wrap)] // the wrap is the point
pub const fn unzigzag(u: u64) -> i64 {
    ((u >> 1) as i64) ^ -((u & 1) as i64)
}

/// Returns the number of bytes needed to encode `value` (1..=9).
///
/// # Examples
///
/// ```
/// assert_eq!(bijoux::i64::encoded_len(0), 1);
/// assert_eq!(bijoux::i64::encoded_len(-124), 1);
/// assert_eq!(bijoux::i64::encoded_len(123), 1);
/// assert_eq!(bijoux::i64::encoded_len(-125), 2);
/// assert_eq!(bijoux::i64::encoded_len(124), 2);
/// assert_eq!(bijoux::i64::encoded_len(i64::MIN), 9);
/// assert_eq!(bijoux::i64::encoded_len(i64::MAX), 9);
/// ```
#[inline]
#[must_use]
pub const fn encoded_len(value: i64) -> usize {
    crate::u64::encoded_len(zigzag(value))
}

/// Encodes `value`, appending the bytes to `buf`.
///
/// # Examples
///
/// ```
/// let mut buf = Vec::new();
/// bijoux::i64::encode(0, &mut buf);
/// bijoux::i64::encode(-1, &mut buf);
/// bijoux::i64::encode(1, &mut buf);
/// assert_eq!(buf, [0x00, 0x01, 0x02]);
/// ```
#[inline]
pub fn encode(value: i64, buf: &mut Vec<u8>) {
    crate::u64::encode(zigzag(value), buf);
}

/// Encodes `value` without allocating, returning an [`EncodedI64`].
///
/// Note that [`EncodedI64`]' `Ord` is byte-lexicographic, which for
/// this signed format is **zigzag order, not numeric order** (see the
/// module docs).
///
/// # Examples
///
/// ```
/// let enc = bijoux::i64::encoded_bytes(-1);
/// assert_eq!(enc.as_slice(), &[0x01]);
/// ```
#[inline]
#[must_use]
pub const fn encoded_bytes(value: i64) -> EncodedI64 {
    crate::u64::encoded_bytes(zigzag(value))
}

/// Decodes one value from the front of `buf`, returning it along with
/// the number of bytes consumed.
///
/// # Errors
///
/// Returns [`DecodeError::BufferTooShort`] if `buf` is shorter than
/// the encoding indicated by its first byte, or
/// [`DecodeError::Overflow`] if a 9-byte encoding's payload exceeds
/// the `u64` (zigzag) range.
///
/// # Examples
///
/// ```
/// let (v, n) = bijoux::i64::decode(&[0x01]).unwrap();
/// assert_eq!((v, n), (-1, 1));
///
/// // Trailing bytes are ignored:
/// let (v, n) = bijoux::i64::decode(&[0x02, 0xFF]).unwrap();
/// assert_eq!((v, n), (1, 1));
/// ```
#[inline]
pub const fn decode(buf: &[u8]) -> Result<(i64, usize), DecodeError> {
    match crate::u64::decode(buf) {
        Ok((value, consumed)) => Ok((unzigzag(value), consumed)),
        Err(err) => Err(err),
    }
}

/// Returns a lazy iterator decoding every value in `buf`.
///
/// The iterator yields `Result<i64, DecodeError>` and fuses after the
/// first error (matching [`crate::u64::decode_iter`]).
///
/// # Examples
///
/// ```
/// let mut buf = Vec::new();
/// for v in [-2i64, -1, 0, 1, 2] {
///     bijoux::i64::encode(v, &mut buf);
/// }
/// let decoded: Result<Vec<i64>, _> = bijoux::i64::decode_iter(&buf).collect();
/// assert_eq!(decoded.unwrap(), [-2, -1, 0, 1, 2]);
/// ```
#[must_use]
pub const fn decode_iter(buf: &[u8]) -> DecodeIter<'_> {
    DecodeIter {
        inner: crate::u64::decode_iter(buf),
    }
}

/// Lazy decoding iterator returned by [`decode_iter`].
///
/// Wraps [`crate::u64::DecodeIter`], un-zigzagging each value.
#[derive(Debug)]
pub struct DecodeIter<'a> {
    inner: crate::u64::DecodeIter<'a>,
}

impl Iterator for DecodeIter<'_> {
    type Item = Result<i64, DecodeError>;

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

/// Decodes every `bijou64s`-encoded value in `buf`, returning them as
/// a `Vec<i64>`.
///
/// # Errors
///
/// Returns the first [`DecodeError`] encountered; values decoded
/// before the error are discarded (all-or-nothing).
///
/// # Examples
///
/// ```
/// use bijoux::i64::{decode_all, encode};
///
/// let mut buf = Vec::new();
/// for v in [0i64, -42, 300, i64::MIN, i64::MAX] {
///     encode(v, &mut buf);
/// }
/// assert_eq!(decode_all(&buf).unwrap(), vec![0, -42, 300, i64::MIN, i64::MAX]);
/// ```
pub fn decode_all(buf: &[u8]) -> Result<Vec<i64>, DecodeError> {
    decode_iter(buf).collect()
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// Spec test vectors: (value, encoding).
    const VECTORS: &[(i64, &[u8])] = &[
        (0, &[0x00]),
        (-1, &[0x01]),
        (1, &[0x02]),
        (-2, &[0x03]),
        (2, &[0x04]),
        (-124, &[0xF7]),            // zigzag = 247, last 1-byte
        (123, &[0xF6]),             // zigzag = 246
        (124, &[0xF8, 0x00]),       // zigzag = 248, first 2-byte
        (-125, &[0xF8, 0x01]),      // zigzag = 249
        (251, &[0xF8, 0xFE]),       // zigzag = 502, last 2-byte tier value pair
        (-252, &[0xF8, 0xFF]),      // zigzag = 503
        (252, &[0xF9, 0x00, 0x00]), // zigzag = 504, first 3-byte
        (
            i64::MAX,
            &[0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x06],
        ), // zigzag = u64::MAX - 1; payload = (u64::MAX - 1) - OFFSET[8]
        (
            i64::MIN,
            &[0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07],
        ), // zigzag = u64::MAX; payload = u64::MAX - OFFSET[8]
    ];

    #[test]
    fn spec_vectors_encode() {
        for &(value, expected) in VECTORS {
            let mut buf = Vec::new();
            encode(value, &mut buf);
            assert_eq!(buf, expected, "encode({value})");
            assert_eq!(encoded_bytes(value).as_slice(), expected);
            assert_eq!(encoded_len(value), expected.len());
        }
    }

    #[test]
    fn spec_vectors_decode() {
        for &(expected, bytes) in VECTORS {
            let (value, consumed) = decode(bytes).unwrap();
            assert_eq!(value, expected, "decode({bytes:02X?})");
            assert_eq!(consumed, bytes.len());
        }
    }

    #[test]
    fn zigzag_bijection_at_extremes() {
        for value in [0i64, -1, 1, i64::MIN, i64::MAX, i64::MIN + 1, i64::MAX - 1] {
            assert_eq!(unzigzag(zigzag(value)), value);
        }
        assert_eq!(zigzag(i64::MIN), u64::MAX);
        assert_eq!(zigzag(i64::MAX), u64::MAX - 1);
    }

    #[test]
    fn tier_boundaries_round_trip() {
        // Signed preimages of every bijou64 tier edge, both signs.
        let unsigned_boundaries: &[u64] = &[
            0,
            247,
            248,
            503,
            504,
            66_039,
            66_040,
            16_843_255,
            16_843_256,
            4_311_810_551,
            4_311_810_552,
            1_103_823_438_327,
            1_103_823_438_328,
            282_578_800_148_983,
            282_578_800_148_984,
            72_340_172_838_076_919,
            72_340_172_838_076_920,
            u64::MAX,
        ];
        for &u in unsigned_boundaries {
            let value = unzigzag(u);
            let mut buf = Vec::new();
            encode(value, &mut buf);
            assert_eq!(buf.len(), crate::u64::encoded_len(u));
            let (decoded, consumed) = decode(&buf).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, buf.len());
        }
    }

    #[test]
    fn errors_propagate() {
        assert_eq!(decode(&[]), Err(DecodeError::BufferTooShort));
        assert_eq!(decode(&[0xF8]), Err(DecodeError::BufferTooShort));
        // Tier-8 payload above the u64 range.
        assert_eq!(
            decode(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
            Err(DecodeError::Overflow)
        );
    }

    #[test]
    fn decode_iter_fuses_and_decode_all_short_circuits() {
        let mut buf = Vec::new();
        encode(-42, &mut buf);
        buf.push(0xF8); // truncated encoding

        let mut iter = decode_iter(&buf);
        assert_eq!(iter.next(), Some(Ok(-42)));
        assert_eq!(iter.next(), Some(Err(DecodeError::BufferTooShort)));
        assert_eq!(iter.next(), None);

        assert_eq!(decode_all(&buf), Err(DecodeError::BufferTooShort));
    }

    #[test]
    fn byte_order_is_zigzag_order() {
        // Documented caveat: lex order interleaves signs by magnitude.
        let order = [0i64, -1, 1, -2, 2, -124, 124];
        let encoded: Vec<EncodedI64> = order.iter().map(|&v| encoded_bytes(v)).collect();
        for pair in encoded.windows(2) {
            assert!(pair[0] < pair[1], "zigzag order violated");
        }
    }

    #[cfg(feature = "bolero")]
    mod property {
        use super::*;

        #[test]
        #[cfg_attr(miri, ignore)]
        fn round_trip() {
            bolero::check!().with_arbitrary::<i64>().for_each(|&value| {
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
