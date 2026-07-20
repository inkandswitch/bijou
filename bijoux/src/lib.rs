//! _Bijoux_, plural of bijou — the bijou family of bijective,
//! length-prefixed variable-length integer encodings.
//!
//! Each format module ([`bijou32`], [`bijou64`], [`bijou128`], and in
//! future the signed `bijou64s`, …) defines **one canonical encoding
//! for one integer type**, exposed as free functions and gated behind a
//! width feature (`u32` / `u64` / `u128`, all enabled by default). The
//! [`Encode`] / [`Decode`] traits are implemented directly on the
//! integer types — the family's one-format-per-type commitment is what
//! makes `impl Encode for u64` unambiguous.
//!
//! # Examples
//!
//! Method syntax on the integers themselves:
//!
//! ```
//! use bijoux::{Decode, Encode};
//!
//! let mut buf = Vec::new();
//! 300u64.encode(&mut buf);
//!
//! let (value, consumed) = u64::decode(&buf).unwrap();
//! assert_eq!((value, consumed), (300, 2));
//! ```
//!
//! Code generic over any bijou-encodable integer:
//!
//! ```
//! use bijoux::Encode;
//!
//! fn frame<T: Encode>(values: &[T], buf: &mut Vec<u8>) {
//!     for &value in values {
//!         value.encode(buf);
//!     }
//! }
//!
//! let mut buf = Vec::new();
//! frame(&[0u64, 247, 248], &mut buf);
//! assert_eq!(buf.len(), 1 + 1 + 2); // third value crosses into tier 1
//! ```

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

#[cfg(feature = "u128")]
pub mod bijou128;
#[cfg(feature = "u32")]
pub mod bijou32;
#[cfg(feature = "u64")]
pub mod bijou64;

/// Encoding half of a bijou codec, implemented directly on the integer
/// type (e.g. `u64` via `bijou64`).
///
/// Mirrors the free-function surface of the format crates
/// (`encoded_len` / `encode` / `encoded_bytes`).
pub trait Encode: Copy {
    /// Stack-only encoded form (e.g. `bijou64::EncodedBytes`). Yields
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
/// type (e.g. `u64` via `bijou64`).
pub trait Decode: Copy + Sized {
    /// Decode failure (e.g. `bijou64::DecodeError`).
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
    type Encoded = bijou64::EncodedBytes;

    const MAX_BYTES: usize = bijou64::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        bijou64::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        bijou64::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> bijou64::EncodedBytes {
        bijou64::encoded_bytes(self)
    }
}

#[cfg(feature = "u64")]
impl Decode for u64 {
    type Error = bijou64::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(u64, usize), bijou64::DecodeError> {
        bijou64::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<u64>, bijou64::DecodeError> {
        bijou64::decode_all(bytes)
    }
}

#[cfg(feature = "u32")]
impl Encode for u32 {
    type Encoded = bijou32::EncodedBytes;

    const MAX_BYTES: usize = bijou32::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        bijou32::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        bijou32::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> bijou32::EncodedBytes {
        bijou32::encoded_bytes(self)
    }
}

#[cfg(feature = "u32")]
impl Decode for u32 {
    type Error = bijou32::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(u32, usize), bijou32::DecodeError> {
        bijou32::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<u32>, bijou32::DecodeError> {
        bijou32::decode_all(bytes)
    }
}

#[cfg(feature = "u128")]
impl Encode for u128 {
    type Encoded = bijou128::EncodedBytes;

    const MAX_BYTES: usize = bijou128::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        bijou128::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        bijou128::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> bijou128::EncodedBytes {
        bijou128::encoded_bytes(self)
    }
}

#[cfg(feature = "u128")]
impl Decode for u128 {
    type Error = bijou128::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(u128, usize), bijou128::DecodeError> {
        bijou128::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<u128>, bijou128::DecodeError> {
        bijou128::decode_all(bytes)
    }
}

#[cfg(all(test, feature = "u64"))]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::vec;

    /// The trait surface must be indistinguishable from the free
    /// functions it delegates to.
    #[test]
    fn u64_matches_free_functions() {
        for value in [0u64, 1, 247, 248, 503, 504, 66_040, u64::MAX] {
            assert_eq!(value.encoded_len(), bijou64::encoded_len(value));
            assert_eq!(
                value.encoded_bytes().as_ref(),
                bijou64::encoded_bytes(value).as_slice()
            );

            let mut via_trait = Vec::new();
            value.encode(&mut via_trait);
            let mut via_free = Vec::new();
            bijou64::encode(value, &mut via_free);
            assert_eq!(via_trait, via_free);

            assert_eq!(u64::decode(&via_trait), bijou64::decode(&via_free));
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
        assert_eq!(<u64 as Encode>::MAX_BYTES, bijou64::MAX_BYTES);
    }
}
