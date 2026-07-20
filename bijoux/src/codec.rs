//! The [`Encode`] / [`Decode`] traits and their implementations on the
//! primitive integer types.
//!
//! Lives in its own module (re-exported at the crate root) because the
//! format modules are named after the primitives (`crate::u64`, …):
//! inside `lib.rs` those module names shadow the primitive types, so
//! `impl Encode for u64` must be written in a scope that does not
//! declare them.

use alloc::vec::Vec;

/// Encoding half of a bijou codec, implemented directly on the integer
/// type (e.g. `u64` via the `u64` module).
///
/// Mirrors the free-function surface of the format crates
/// (`encoded_len` / `encode` / `encoded_bytes`).
pub trait Encode: Copy {
    /// Stack-only encoded form (e.g. `u64::EncodedU64`). Yields
    /// exactly the encoded bytes — no trailing padding — via `as_ref()`.
    type Encoded: AsRef<[u8]>;

    /// Maximum encoded length in bytes (e.g. 9 for `u64`).
    const MAX_BYTES: usize;

    /// Number of bytes `self` encodes to (`1..=MAX_BYTES`).
    #[must_use]
    fn encoded_len(self) -> usize;

    /// Append the encoding of `self` to `buf`.
    fn encode(self, buf: &mut Vec<u8>);

    /// Encode `self` without allocating.
    #[must_use]
    fn encoded_bytes(self) -> Self::Encoded;
}

/// Decoding half of a bijou codec, implemented directly on the integer
/// type (e.g. `u64` via the `u64` module).
pub trait Decode: Copy + Sized {
    /// Decode failure (e.g. `u64::DecodeError`).
    type Error;

    /// Decode one value from the front of `bytes`, returning it with
    /// the number of bytes consumed.
    ///
    /// # Errors
    ///
    /// Returns [`Decode::Error`] when `bytes` does not begin with a
    /// valid encoding (e.g. truncated input, or a payload overflowing
    /// `Self`).
    fn decode(bytes: &[u8]) -> Result<(Self, usize), Self::Error>;

    /// Decode every value in `bytes`, all-or-nothing.
    ///
    /// The default implementation loops [`Decode::decode`]; formats may
    /// override with something faster.
    ///
    /// # Errors
    ///
    /// Returns the first [`Decode::Error`] encountered; prior values
    /// are discarded.
    fn decode_all(bytes: &[u8]) -> Result<Vec<Self>, Self::Error> {
        let mut values = Vec::new();
        let mut remaining = bytes;
        while !remaining.is_empty() {
            let (value, consumed) = Self::decode(remaining)?;
            values.push(value);
            remaining = remaining.get(consumed..).unwrap_or_default();
        }
        Ok(values)
    }
}

#[cfg(feature = "u64")]
impl Encode for u64 {
    type Encoded = crate::u64::EncodedU64;

    const MAX_BYTES: usize = crate::u64::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        crate::u64::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        crate::u64::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> crate::u64::EncodedU64 {
        crate::u64::encoded_bytes(self)
    }
}

#[cfg(feature = "u64")]
impl Decode for u64 {
    type Error = crate::u64::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(u64, usize), crate::u64::DecodeError> {
        crate::u64::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<u64>, crate::u64::DecodeError> {
        crate::u64::decode_all(bytes)
    }
}

#[cfg(feature = "u32")]
impl Encode for u32 {
    type Encoded = crate::u32::EncodedU32;

    const MAX_BYTES: usize = crate::u32::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        crate::u32::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        crate::u32::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> crate::u32::EncodedU32 {
        crate::u32::encoded_bytes(self)
    }
}

#[cfg(feature = "u32")]
impl Decode for u32 {
    type Error = crate::u32::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(u32, usize), crate::u32::DecodeError> {
        crate::u32::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<u32>, crate::u32::DecodeError> {
        crate::u32::decode_all(bytes)
    }
}

#[cfg(feature = "u128")]
impl Encode for u128 {
    type Encoded = crate::u128::EncodedU128;

    const MAX_BYTES: usize = crate::u128::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        crate::u128::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        crate::u128::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> crate::u128::EncodedU128 {
        crate::u128::encoded_bytes(self)
    }
}

#[cfg(feature = "u128")]
impl Decode for u128 {
    type Error = crate::u128::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(u128, usize), crate::u128::DecodeError> {
        crate::u128::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<u128>, crate::u128::DecodeError> {
        crate::u128::decode_all(bytes)
    }
}

#[cfg(feature = "i64")]
impl Encode for i64 {
    type Encoded = crate::i64::EncodedI64;

    const MAX_BYTES: usize = crate::i64::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        crate::i64::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        crate::i64::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> crate::i64::EncodedI64 {
        crate::i64::encoded_bytes(self)
    }
}

#[cfg(feature = "i64")]
impl Decode for i64 {
    type Error = crate::i64::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(i64, usize), crate::i64::DecodeError> {
        crate::i64::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<i64>, crate::i64::DecodeError> {
        crate::i64::decode_all(bytes)
    }
}

#[cfg(all(test, feature = "u64"))]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    #[cfg(feature = "i64")]
    fn i64_matches_free_functions() {
        for value in [0i64, -1, 1, -124, 123, -125, 124, i64::MIN, i64::MAX] {
            assert_eq!(value.encoded_len(), crate::i64::encoded_len(value));

            let mut via_trait = Vec::new();
            value.encode(&mut via_trait);
            let mut via_free = Vec::new();
            crate::i64::encode(value, &mut via_free);
            assert_eq!(via_trait, via_free);

            assert_eq!(i64::decode(&via_trait), crate::i64::decode(&via_free));
        }
    }

    /// The trait surface must be indistinguishable from the free
    /// functions it delegates to.
    #[test]
    fn u64_matches_free_functions() {
        for value in [0u64, 1, 247, 248, 503, 504, 66_040, u64::MAX] {
            assert_eq!(value.encoded_len(), crate::u64::encoded_len(value));
            assert_eq!(
                value.encoded_bytes().as_ref(),
                crate::u64::encoded_bytes(value).as_slice()
            );

            let mut via_trait = Vec::new();
            value.encode(&mut via_trait);
            let mut via_free = Vec::new();
            crate::u64::encode(value, &mut via_free);
            assert_eq!(via_trait, via_free);

            assert_eq!(u64::decode(&via_trait), crate::u64::decode(&via_free));
        }
    }

    #[test]
    fn u64_decode_all_roundtrip() {
        let values = vec![0u64, 42, 300, u64::MAX];
        let mut buf = Vec::new();
        for &value in &values {
            value.encode(&mut buf);
        }

        assert_eq!(u64::decode_all(&buf).expect("valid buffer"), values);
    }

    #[test]
    fn u64_max_bytes_matches() {
        assert_eq!(<u64 as Encode>::MAX_BYTES, crate::u64::MAX_BYTES);
    }
}
