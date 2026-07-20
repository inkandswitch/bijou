//! _Bijoux_, plural of bijou — the bijou family of bijective,
//! length-prefixed variable-length integer encodings.
//!
//! Each format module ([`u32`](crate::u32), [`u64`](crate::u64), [`u128`](crate::u128), and in
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
pub mod u128;
#[cfg(feature = "u32")]
pub mod u32;
#[cfg(feature = "u64")]
pub mod u64;

mod primitive_impls;

/// Encoding half of a bijou codec, implemented directly on the integer
/// type (e.g. `u64` via the `u64` module).
///
/// Mirrors the free-function surface of the format crates
/// (`encoded_len` / `encode` / `encoded_bytes`).
pub trait Encode: Copy {
    /// Stack-only encoded form (e.g. `u64::EncodedBytes`). Yields
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
