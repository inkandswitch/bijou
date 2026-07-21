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

#[cfg(feature = "i32")]
impl Encode for i32 {
    type Encoded = crate::i32::EncodedI32;

    const MAX_BYTES: usize = crate::i32::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        crate::i32::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        crate::i32::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> crate::i32::EncodedI32 {
        crate::i32::encoded_bytes(self)
    }
}

#[cfg(feature = "i32")]
impl Decode for i32 {
    type Error = crate::i32::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(i32, usize), crate::i32::DecodeError> {
        crate::i32::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<i32>, crate::i32::DecodeError> {
        crate::i32::decode_all(bytes)
    }
}

#[cfg(feature = "i128")]
impl Encode for i128 {
    type Encoded = crate::i128::EncodedI128;

    const MAX_BYTES: usize = crate::i128::MAX_BYTES;

    #[inline]
    fn encoded_len(self) -> usize {
        crate::i128::encoded_len(self)
    }

    #[inline]
    fn encode(self, buf: &mut Vec<u8>) {
        crate::i128::encode(self, buf);
    }

    #[inline]
    fn encoded_bytes(self) -> crate::i128::EncodedI128 {
        crate::i128::encoded_bytes(self)
    }
}

#[cfg(feature = "i128")]
impl Decode for i128 {
    type Error = crate::i128::DecodeError;

    #[inline]
    fn decode(bytes: &[u8]) -> Result<(i128, usize), crate::i128::DecodeError> {
        crate::i128::decode(bytes)
    }

    #[inline]
    fn decode_all(bytes: &[u8]) -> Result<Vec<i128>, crate::i128::DecodeError> {
        crate::i128::decode_all(bytes)
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

    /// The trait surface must be indistinguishable from the free
    /// functions it delegates to — one macro-generated test per width
    /// so a copy-paste slip in any of the six delegation blocks (e.g.
    /// `i32` delegating to `crate::u32`) cannot hide.
    macro_rules! matches_free_functions {
        ($name:ident, $ty:ty, $module:ident, $feature:literal, $values:expr) => {
            #[test]
            #[cfg(feature = $feature)]
            fn $name() {
                for value in $values {
                    assert_eq!(value.encoded_len(), crate::$module::encoded_len(value));
                    assert_eq!(
                        value.encoded_bytes().as_ref(),
                        crate::$module::encoded_bytes(value).as_slice()
                    );

                    let mut via_trait = Vec::new();
                    value.encode(&mut via_trait);
                    let mut via_free = Vec::new();
                    crate::$module::encode(value, &mut via_free);
                    assert_eq!(via_trait, via_free);

                    assert_eq!(
                        <$ty as Decode>::decode(&via_trait),
                        crate::$module::decode(&via_free)
                    );
                }
            }
        };
    }

    matches_free_functions!(
        u32_matches_free_functions,
        u32,
        u32,
        "u32",
        [0u32, 1, 251, 252, 507, 508, u32::MAX]
    );
    matches_free_functions!(
        u64_matches_free_functions,
        u64,
        u64,
        "u64",
        [0u64, 1, 247, 248, 503, 504, 66_040, u64::MAX]
    );
    matches_free_functions!(
        u128_matches_free_functions,
        u128,
        u128,
        "u128",
        [0u128, 1, 239, 240, 495, 496, u128::MAX]
    );
    matches_free_functions!(
        i32_matches_free_functions,
        i32,
        i32,
        "i32",
        [0i32, -1, 1, -126, 125, -127, 126, i32::MIN, i32::MAX]
    );
    matches_free_functions!(
        i64_matches_free_functions,
        i64,
        i64,
        "i64",
        [0i64, -1, 1, -124, 123, -125, 124, i64::MIN, i64::MAX]
    );
    matches_free_functions!(
        i128_matches_free_functions,
        i128,
        i128,
        "i128",
        [0i128, -1, 1, -120, 119, -121, 120, i128::MIN, i128::MAX]
    );

    #[test]
    fn u64_decode_all_roundtrip() {
        let values = vec![0u64, 42, 300, u64::MAX];
        let mut buf = Vec::new();
        for &value in &values {
            value.encode(&mut buf);
        }

        assert_eq!(u64::decode_all(&buf).expect("valid buffer"), values);
    }

    /// Pin the literal widths so drift on either side of the
    /// delegation fails (asserting `impl == module` would be a
    /// tautology — the impl reads the module const).
    #[test]
    fn max_bytes_literals() {
        assert_eq!(<u32 as Encode>::MAX_BYTES, 5);
        assert_eq!(<u64 as Encode>::MAX_BYTES, 9);
        assert_eq!(<u128 as Encode>::MAX_BYTES, 17);
        #[cfg(feature = "i32")]
        assert_eq!(<i32 as Encode>::MAX_BYTES, 5);
        #[cfg(feature = "i64")]
        assert_eq!(<i64 as Encode>::MAX_BYTES, 9);
        #[cfg(feature = "i128")]
        assert_eq!(<i128 as Encode>::MAX_BYTES, 17);
    }

    /// Exercise the trait's *default* `decode_all` body (every real
    /// impl overrides it) via a minimal in-test decoder, including the
    /// exact-buffer-boundary and malformed-tail paths.
    #[test]
    fn decode_all_default_body() {
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct Byte(u8);

        impl Decode for Byte {
            type Error = ();

            fn decode(bytes: &[u8]) -> Result<(Self, usize), ()> {
                match bytes.first() {
                    Some(&0xFF) | None => Err(()),
                    Some(&b) => Ok((Byte(b), 1)),
                }
            }
            // No decode_all override: the default body runs.
        }

        // Values end exactly at the buffer boundary.
        assert_eq!(
            Byte::decode_all(&[1, 2, 3]),
            Ok(vec![Byte(1), Byte(2), Byte(3)])
        );
        // Empty input decodes to nothing.
        assert_eq!(Byte::decode_all(&[]), Ok(vec![]));
        // Malformed tail short-circuits (all-or-nothing).
        assert_eq!(Byte::decode_all(&[1, 0xFF, 2]), Err(()));
    }
}
