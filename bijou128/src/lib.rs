//! Bijective variable-length encoding for unsigned 128-bit integers.
//!
//! bijou128 (**BIJ**ective **O**ffset **U128**) encodes `u128` values into
//! 1–17 bytes using a tag-byte prefix scheme derived from [VARU64], modified
//! with per-tier offsets to achieve **structural canonicality** — each value
//! has exactly one encoding, and each encoding has exactly one value. This
//! is [bijective numeration] applied to VARU64's tag-byte framing, widened
//! to cover the full `u128` range.
//!
//! bijou128 is the wider sibling of [`bijou64`]: same recurrence, same
//! big-endian payload layout, same canonical-by-construction property. The
//! only structural difference is the tag-byte threshold (`240` instead of
//! `248`) which reserves the upper 16 tag values (`0xF0..=0xFF`) for the 16
//! multi-byte tiers needed to span 128 bits.
//!
//! # Encoding
//!
//! The first byte determines the encoding:
//!
//! - `0x00..=0xEF` (0–239): the byte _is_ the value. One byte total.
//! - `0xF0..=0xFF` (240–255): length tag. Additional bytes = `tag - 239`.
//!   Payload is big-endian `value - OFFSET[tier]`.
//!
//! The offset for each tier is the first value not representable by the
//! previous tier, making all tier ranges disjoint by construction:
//!
//! ```text
//! ┌───────────┬──────────────────┬───────────────────────────────────────┐
//! │ Tag       │ Additional bytes │ Tier offset (first value in tier)     │
//! ├───────────┼──────────────────┼───────────────────────────────────────┤
//! │ 0x00-0xEF │ 0                │ 0                                     │
//! │ 0xF0      │ 1                │ 240                                   │
//! │ 0xF1      │ 2                │ 496                                   │
//! │ 0xF2      │ 3                │ 66,032                                │
//! │ 0xF3      │ 4                │ 16,843,248                            │
//! │ 0xF4      │ 5                │ 4,311,810,544                         │
//! │ 0xF5      │ 6                │ ≈ 1.1 × 10^12                         │
//! │ ...       │ ...              │ ...                                   │
//! │ 0xFF      │ 16               │ Σ 256^k for k in [1, 15], plus 240    │
//! └───────────┴──────────────────┴───────────────────────────────────────┘
//! ```
//!
//! See [`OFFSETS`] for the exact per-tier offsets, and the
//! [specification](https://github.com/inkandswitch/bijou/blob/main/bijou128/SPEC.md)
//! for the full format definition, design rationale, and test vectors.
//!
//! # Canonicality
//!
//! Unlike [VARU64], which requires a runtime check to reject overlong
//! encodings, bijou128 achieves canonicality structurally: each tier's
//! value range is disjoint, so no byte sequence can decode to a value
//! representable in a shorter form. The only decoder error conditions are
//! buffer underflow and arithmetic overflow on tier 16.
//!
//! # Examples
//!
//! ```
//! // Single-byte tier-0 encoding.
//! let mut buf = Vec::new();
//! bijou128::encode(42, &mut buf);
//! assert_eq!(buf, [0x2A]);
//!
//! // First multi-byte value: tag 0xF0 plus 1 payload byte.
//! let mut buf = Vec::new();
//! bijou128::encode(240, &mut buf);
//! assert_eq!(buf, [0xF0, 0x00]);
//!
//! // Round-trip through `decode`.
//! let mut buf = Vec::new();
//! bijou128::encode(u128::MAX, &mut buf);
//! let (value, len) = bijou128::decode(&buf).unwrap();
//! assert_eq!(value, u128::MAX);
//! assert_eq!(len, 17);
//! ```
//!
//! # Family
//!
//! bijou128 is one of three width-specialised siblings sharing the same
//! recurrence, big-endian payload layout, and canonical-by-construction
//! property. They differ only in the tag-byte threshold and tier count:
//!
//! - [`bijou32`] — narrower `u32` variant (1–5 bytes, threshold `252`).
//! - [`bijou64`] — `u64` variant (1–9 bytes, threshold `248`).
//!
//! [VARU64]: https://github.com/AljoschaMeyer/varu64-rs
//! [bijective numeration]: https://en.wikipedia.org/wiki/Bijective_numeration
//! [`bijou32`]: https://docs.rs/bijou32
//! [`bijou64`]: https://docs.rs/bijou64

#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

#[allow(unused_imports)] // vec! macro used in tests
use alloc::{vec, vec::Vec};

/// Maximum number of bytes a `bijou128` encoding can occupy.
pub const MAX_BYTES: usize = 17;

/// Tag byte threshold: values below this are encoded as a single byte.
///
/// Compare with `bijou64::TAG_THRESHOLD = 248`; bijou128 reserves 16 tag
/// values (`0xF0..=0xFF`) instead of 8, since spanning 128 bits requires
/// payloads up to 16 bytes wide.
const TAG_THRESHOLD: u8 = 240;

/// Number of multi-byte tiers (tags 240–255).
const NUM_TIERS: usize = 16;

/// Computes the tier offset for tier `n`.
///
/// Each tier's offset is the first value not representable by the previous
/// tier. Recurrence: `offset(n) = offset(n-1) + 256^(n-1)` for `n >= 2`,
/// with `offset(1) = 240` and `offset(0) = 0`.
const fn tier_offset(n: usize) -> u128 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return TAG_THRESHOLD as u128;
    }

    let mut result = TAG_THRESHOLD as u128;
    let mut power = 1u128; // 256^0
    let mut i = 2;
    while i <= n {
        power = power.saturating_mul(256);
        result = result.saturating_add(power);
        i += 1;
    }
    result
}

/// Per-tier offsets.
///
/// `OFFSETS[t]` is the first value that requires tier `t` (1-indexed).
/// Index 0 is unused (tier 0 values are encoded as the tag byte itself).
const OFFSETS: [u128; NUM_TIERS + 1] = [
    tier_offset(0),
    tier_offset(1),
    tier_offset(2),
    tier_offset(3),
    tier_offset(4),
    tier_offset(5),
    tier_offset(6),
    tier_offset(7),
    tier_offset(8),
    tier_offset(9),
    tier_offset(10),
    tier_offset(11),
    tier_offset(12),
    tier_offset(13),
    tier_offset(14),
    tier_offset(15),
    tier_offset(16),
];

/// Per-tier upper bounds (exclusive).
///
/// A value belongs to tier `t` if `OFFSETS[t] <= value < BOUNDS[t]`.
/// `BOUNDS[t] == OFFSETS[t + 1]` for tiers 1–15. Tier 16 extends to
/// `u128::MAX` (the decoder handles overflow via `checked_add`).
const BOUNDS: [u128; NUM_TIERS + 1] = [
    tier_offset(1), // tier 0 upper bound = tier 1 offset
    tier_offset(2),
    tier_offset(3),
    tier_offset(4),
    tier_offset(5),
    tier_offset(6),
    tier_offset(7),
    tier_offset(8),
    tier_offset(9),
    tier_offset(10),
    tier_offset(11),
    tier_offset(12),
    tier_offset(13),
    tier_offset(14),
    tier_offset(15),
    tier_offset(16),
    u128::MAX, // tier 16 extends to u128::MAX
];

/// Returns the encoded length of `value` in bytes (1–17).
///
/// Uses `leading_zeros` (a single `lzcnt`/`clz` instruction on most
/// architectures) to derive a candidate tier from the value's bit-width,
/// then applies at most one comparison to correct for the per-tier
/// offsets. This replaces a 16-arm if/else chain with O(1) arithmetic.
///
/// # Examples
///
/// ```
/// assert_eq!(bijou128::encoded_len(0), 1);
/// assert_eq!(bijou128::encoded_len(239), 1);
/// assert_eq!(bijou128::encoded_len(240), 2);
/// assert_eq!(bijou128::encoded_len(495), 2);
/// assert_eq!(bijou128::encoded_len(496), 3);
/// assert_eq!(bijou128::encoded_len(u128::MAX), 17);
/// ```
#[inline]
#[must_use]
pub const fn encoded_len(value: u128) -> usize {
    // Fast path: tier 0 values (0–239) are the most common in many
    // workloads and need only a single well-predicted comparison.
    if value < BOUNDS[0] {
        return 1;
    }

    // For multi-byte tiers, derive the tier from the value's bit-width
    // via `leading_zeros` (a single `lzcnt`/`clz` instruction on most
    // architectures), then correct with one comparison.
    let bw = 128 - value.leading_zeros(); // u32, 8..=128 here

    // Same formula as bijou64: `(bw - 1) / 8 + 2`.
    //   bw=8     -> candidate 2  (tier 1, 2 bytes total)
    //   bw=9..16 -> candidate 3  (tier 2, 3 bytes total)
    //   ...
    //   bw=128   -> candidate 17 (tier 16, 17 bytes total)
    let candidate = ((bw - 1) / 8 + 2) as usize;

    // The candidate can be at most 1 too high because bijou128's tier
    // offsets push the boundary slightly past each power-of-256.
    // One comparison corrects for boundary values.
    //
    // SAFETY (indexing): `candidate ∈ [2, 17]` because `bw ∈ [8, 128]` after
    // the tier-0 early return, so `candidate - 2 ∈ [0, 15]` — always in
    // bounds for the 17-element `BOUNDS` array.
    #[allow(clippy::indexing_slicing)]
    if value < BOUNDS[candidate - 2] {
        candidate - 1
    } else {
        candidate
    }
}

/// Encodes `value` as a `bijou128`, appending bytes to `buf`.
///
/// # Examples
///
/// ```
/// let mut buf = Vec::new();
/// bijou128::encode(42, &mut buf);
/// assert_eq!(buf, [0x2A]);
///
/// buf.clear();
/// bijou128::encode(240, &mut buf);
/// assert_eq!(buf, [0xF0, 0x00]);
/// ```
#[allow(clippy::cast_possible_truncation, clippy::indexing_slicing)]
pub fn encode(value: u128, buf: &mut Vec<u8>) {
    if value < BOUNDS[0] {
        buf.push((value & 0xFF) as u8);
        return;
    }

    let bw = 128 - value.leading_zeros();
    let mut tier = ((bw - 1) / 8 + 1) as usize;
    if value < BOUNDS[tier - 1] {
        tier -= 1;
    }

    let tag = (TAG_THRESHOLD as usize + tier - 1) as u8;
    let payload = (value - OFFSETS[tier]) << (8 * (NUM_TIERS - tier));
    let pb = payload.to_be_bytes();

    let original_len = buf.len();
    buf.extend_from_slice(&[
        tag, pb[0], pb[1], pb[2], pb[3], pb[4], pb[5], pb[6], pb[7], pb[8], pb[9], pb[10], pb[11],
        pb[12], pb[13], pb[14], pb[15],
    ]);
    buf.truncate(original_len + tier + 1);
}

/// A stack-allocated bijou128 encoding, carrying its own valid length.
///
/// Returned by [`encoded_bytes`]. Always exposes the correct byte
/// slice via `Deref<Target = [u8]>` and `AsRef<[u8]>`, so callers can
/// never accidentally read past the encoded prefix.
///
/// # Examples
///
/// ```
/// use bijou128::encoded_bytes;
///
/// let enc = encoded_bytes(240);
/// assert_eq!(&*enc, &[0xF0, 0x00]);   // Deref<Target = [u8]>
/// assert_eq!(enc.len(), 2);
/// assert_eq!(enc.as_ref(), &[0xF0, 0x00][..]);
///
/// // Works wherever `&[u8]` is accepted:
/// fn send(bytes: &[u8]) -> usize { bytes.len() }
/// assert_eq!(send(&enc), 2);
///
/// // Iterate the encoded bytes:
/// let collected: Vec<u8> = enc.into_iter().collect();
/// assert_eq!(collected, [0xF0, 0x00]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EncodedBytes {
    buf: [u8; MAX_BYTES],
    /// Invariant: `len <= MAX_BYTES`. Stored as `u8` because the value
    /// is always in `1..=17`; the smaller width makes `EncodedBytes`
    /// fit in 18 bytes total and `Copy` cheaper.
    len: u8,
}

// We implement `PartialEq`, `Eq`, `Hash`, `PartialOrd`, and `Ord` by
// hand against the encoded byte slice (`self.as_slice()`) rather than
// deriving them on the `(buf, len)` pair. Same rationale as bijou64:
// the trailing `MAX_BYTES - len` bytes of `buf` are always zero, so a
// derived `PartialEq` *would* match — but only because we maintain
// that zero-padding invariant. Comparing the live slice removes that
// subtle coupling, and explicitly compares via the natural lex order
// that bijou guarantees matches numeric order.

impl PartialEq for EncodedBytes {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for EncodedBytes {}

impl core::hash::Hash for EncodedBytes {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl PartialOrd for EncodedBytes {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EncodedBytes {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Bijou's lex-order property: byte-lex order = numeric order.
        self.as_slice().cmp(other.as_slice())
    }
}

impl EncodedBytes {
    /// Length of the encoding in bytes (always in `1..=MAX_BYTES`).
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Always `false`. Bijou encodings are never empty; the smallest
    /// is a single-byte encoding of value `0`. Included to satisfy
    /// the standard `len`/`is_empty` API pairing.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Returns the encoded bytes as a `&[u8]` of the correct length.
    ///
    /// Equivalent to `&*self` or `self.as_ref()`; provided as an
    /// explicit method for callers that prefer it over `Deref`.
    #[inline]
    #[must_use]
    #[allow(clippy::indexing_slicing)] // len is invariant <= MAX_BYTES
    pub const fn as_slice(&self) -> &[u8] {
        self.buf.split_at(self.len as usize).0
    }
}

impl core::ops::Deref for EncodedBytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for EncodedBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl core::borrow::Borrow<[u8]> for EncodedBytes {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl IntoIterator for EncodedBytes {
    type Item = u8;
    type IntoIter = core::iter::Take<core::array::IntoIter<u8, MAX_BYTES>>;

    fn into_iter(self) -> Self::IntoIter {
        let len = self.len();
        self.buf.into_iter().take(len)
    }
}

impl<'a> IntoIterator for &'a EncodedBytes {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

/// Encodes `value` as a stack-allocated [`EncodedBytes`].
///
/// This is the alloc-free encoding entry point — the returned value
/// dereferences directly to a `&[u8]` of the correct length, so the
/// caller can pass it anywhere a byte slice is accepted without
/// tracking the length separately.
///
/// Uses `leading_zeros` to derive the tier in O(1) rather than walking
/// an if/else chain. See [`encoded_len`] for the same technique.
///
/// # Examples
///
/// ```
/// let enc = bijou128::encoded_bytes(240);
/// assert_eq!(&*enc, &[0xF0, 0x00]);
///
/// // Use it anywhere a byte slice is accepted:
/// let mut sink = Vec::new();
/// sink.extend_from_slice(&enc);
/// assert_eq!(sink, [0xF0, 0x00]);
/// ```
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::indexing_slicing)]
pub const fn encoded_bytes(value: u128) -> EncodedBytes {
    if value < BOUNDS[0] {
        return EncodedBytes {
            buf: [
                (value & 0xFF) as u8,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            len: 1,
        };
    }

    let bw = 128 - value.leading_zeros();
    let mut tier = ((bw - 1) / 8 + 1) as usize;
    if value < BOUNDS[tier - 1] {
        tier -= 1;
    }

    // Shift the payload so its `tier` significant bytes occupy the
    // high `tier` bytes of a u128. After `to_be_bytes()` those bytes
    // land at positions 0..tier, with zeros at positions tier..16 — so
    // the entire 17-byte array can be constructed as one fixed-shape
    // literal that LLVM compiles to a single `bswap` + 17-byte store.
    let tag = (TAG_THRESHOLD as usize + tier - 1) as u8;
    let payload = (value - OFFSETS[tier]) << (8 * (NUM_TIERS - tier));
    let pb = payload.to_be_bytes();

    EncodedBytes {
        buf: [
            tag, pb[0], pb[1], pb[2], pb[3], pb[4], pb[5], pb[6], pb[7], pb[8], pb[9], pb[10],
            pb[11], pb[12], pb[13], pb[14], pb[15],
        ],
        // Invariant: `tier ∈ [1, 16]` and `2 <= tier + 1 <= MAX_BYTES = 17`,
        // so the cast cannot truncate.
        len: (tier + 1) as u8,
    }
}

/// Decodes a `bijou128` from the front of `buf`.
///
/// Returns `(value, bytes_consumed)` on success.
///
/// # Errors
///
/// - [`DecodeError::BufferTooShort`] if `buf` has fewer bytes than the
///   encoding requires.
/// - [`DecodeError::Overflow`] if the input is a tier-16 encoding
///   (first byte `0xFF`) whose 16-byte payload, added to the tier-16
///   offset, would exceed `u128::MAX`.
///
/// # Examples
///
/// ```
/// // Single-byte value
/// let (v, n) = bijou128::decode(&[0x2A]).unwrap();
/// assert_eq!((v, n), (42, 1));
///
/// // Multi-byte value with trailing data
/// let (v, n) = bijou128::decode(&[0xF0, 0x00, 0xFF]).unwrap();
/// assert_eq!((v, n), (240, 2));
/// ```
#[inline]
// 17 tiers means a 17-arm slice-pattern match; the per-arm bodies are
// trivially small but the function as a whole exceeds the default
// clippy::too_many_lines threshold. Splitting it would force a
// runtime-length copy (see bijou64's `OPTIMISATION.md`), so we keep
// the explicit per-tier dispatch.
#[allow(clippy::many_single_char_names, clippy::too_many_lines)]
pub const fn decode(buf: &[u8]) -> Result<(u128, usize), DecodeError> {
    let Some((&tag, rest)) = buf.split_first() else {
        return Err(DecodeError::BufferTooShort);
    };

    if tag < TAG_THRESHOLD {
        return Ok((tag as u128, 1));
    }

    // Read big-endian payload and add tier offset. Slice-pattern matching
    // proves to the compiler that enough bytes exist in each arm, and
    // `u128::from_be_bytes` reconstructs the payload without manual shifts.
    //
    // NOTE: bijou64 documents that consolidating these into a single
    // tier-dispatched while-loop pad regressed decode 4–7× on Zen 5 because
    // LLVM specialises each arm here to a fixed-size copy. The same
    // expectation applies at 128 bits — keep the per-tier arms.
    let (offset, payload, consumed) = match tag {
        0xF0 => match rest {
            &[a, ..] => (
                OFFSETS[1],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a]),
                2,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF1 => match rest {
            &[a, b, ..] => (
                OFFSETS[2],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b]),
                3,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF2 => match rest {
            &[a, b, c, ..] => (
                OFFSETS[3],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c]),
                4,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF3 => match rest {
            &[a, b, c, d, ..] => (
                OFFSETS[4],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d]),
                5,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF4 => match rest {
            &[a, b, c, d, e, ..] => (
                OFFSETS[5],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d, e]),
                6,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF5 => match rest {
            &[a, b, c, d, e, f, ..] => (
                OFFSETS[6],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d, e, f]),
                7,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF6 => match rest {
            &[a, b, c, d, e, f, g, ..] => (
                OFFSETS[7],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d, e, f, g]),
                8,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF7 => match rest {
            &[a, b, c, d, e, f, g, h, ..] => (
                OFFSETS[8],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, 0, a, b, c, d, e, f, g, h]),
                9,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF8 => match rest {
            &[a, b, c, d, e, f, g, h, i, ..] => (
                OFFSETS[9],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, 0, a, b, c, d, e, f, g, h, i]),
                10,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xF9 => match rest {
            &[a, b, c, d, e, f, g, h, i, j, ..] => (
                OFFSETS[10],
                u128::from_be_bytes([0, 0, 0, 0, 0, 0, a, b, c, d, e, f, g, h, i, j]),
                11,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xFA => match rest {
            &[a, b, c, d, e, f, g, h, i, j, k, ..] => (
                OFFSETS[11],
                u128::from_be_bytes([0, 0, 0, 0, 0, a, b, c, d, e, f, g, h, i, j, k]),
                12,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xFB => match rest {
            &[a, b, c, d, e, f, g, h, i, j, k, l, ..] => (
                OFFSETS[12],
                u128::from_be_bytes([0, 0, 0, 0, a, b, c, d, e, f, g, h, i, j, k, l]),
                13,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xFC => match rest {
            &[a, b, c, d, e, f, g, h, i, j, k, l, m, ..] => (
                OFFSETS[13],
                u128::from_be_bytes([0, 0, 0, a, b, c, d, e, f, g, h, i, j, k, l, m]),
                14,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xFD => match rest {
            &[a, b, c, d, e, f, g, h, i, j, k, l, m, n, ..] => (
                OFFSETS[14],
                u128::from_be_bytes([0, 0, a, b, c, d, e, f, g, h, i, j, k, l, m, n]),
                15,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        0xFE => match rest {
            &[a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, ..] => (
                OFFSETS[15],
                u128::from_be_bytes([0, a, b, c, d, e, f, g, h, i, j, k, l, m, n, o]),
                16,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
        // 0xFF — only remaining value since tag >= TAG_THRESHOLD (240)
        _ => match rest {
            &[a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, ..] => (
                OFFSETS[16],
                u128::from_be_bytes([a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p]),
                17,
            ),
            _ => return Err(DecodeError::BufferTooShort),
        },
    };

    match offset.checked_add(payload) {
        Some(value) => Ok((value, consumed)),
        None => Err(DecodeError::Overflow),
    }
}

/// Iterator over the bijou128-encoded values in `buf`.
///
/// Calling [`decode_iter`] is equivalent to writing the manual cursor
/// loop yourself; it's a convenience for callers who'd rather work
/// with the iterator combinators than slice arithmetic.
///
/// # Behaviour
///
/// - Each `next()` decodes one value at the current cursor position.
/// - On success, yields `Some(Ok(value))` and advances the cursor by
///   the consumed-byte count.
/// - On a decode error (`BufferTooShort` or `Overflow`), yields
///   `Some(Err(_))` and then `None` on every subsequent call (the
///   iterator is fused — it does not retry).
/// - When the input is fully consumed, yields `None`.
///
/// # Examples
///
/// ```
/// use bijou128::{decode_iter, encode};
///
/// let mut buf = Vec::new();
/// for v in [0u128, 42, 500, 65_535] {
///     encode(v, &mut buf);
/// }
///
/// // Collect into a Result<Vec<_>, _> to short-circuit on the first error.
/// let decoded: Result<Vec<u128>, _> = decode_iter(&buf).collect();
/// assert_eq!(decoded.unwrap(), [0, 42, 500, 65_535]);
/// ```
#[derive(Debug)]
#[must_use = "iterators are lazy; consume with for/.collect/.next"]
pub struct DecodeIter<'a> {
    cursor: &'a [u8],
    fused_err: bool,
}

/// Decodes successive `bijou128`-encoded values from `buf`.
///
/// Returns an iterator that yields `Result<u128, DecodeError>` for each
/// encoded value in `buf`, in order. See [`DecodeIter`] for the exact
/// fused-iteration semantics.
#[inline]
pub const fn decode_iter(buf: &[u8]) -> DecodeIter<'_> {
    DecodeIter {
        cursor: buf,
        fused_err: false,
    }
}

impl Iterator for DecodeIter<'_> {
    type Item = Result<u128, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fused_err || self.cursor.is_empty() {
            return None;
        }
        match decode(self.cursor) {
            Ok((value, consumed)) => {
                // SAFETY (indexing): `decode` returns `consumed` only on
                // success, and `consumed <= cursor.len()` is enforced by
                // its internal slice-pattern matches. So splitting at
                // `consumed` is always in bounds.
                self.cursor = match self.cursor.get(consumed..) {
                    Some(rest) => rest,
                    None => &[],
                };
                Some(Ok(value))
            }
            Err(err) => {
                self.fused_err = true;
                Some(Err(err))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.fused_err || self.cursor.is_empty() {
            return (0, Some(0));
        }
        // Lower bound: at least one more `next()` will produce something
        // (an Ok or an Err) since the cursor is non-empty.
        // Upper bound: every remaining value takes at least 1 byte, so
        // we'll yield at most `cursor.len()` more items.
        (1, Some(self.cursor.len()))
    }
}

impl core::iter::FusedIterator for DecodeIter<'_> {}

/// Decodes every `bijou128`-encoded value in `buf`, returning them as
/// a `Vec<u128>`.
///
/// Equivalent to `decode_iter(buf).collect()` but spells out the
/// short-circuit collect pattern explicitly. The lazy iterator API
/// ([`decode_iter`]) is more appropriate when you might stop early or
/// want to combine with `take`, `filter`, etc.
///
/// # Errors
///
/// Returns the first [`DecodeError`] encountered. The values
/// successfully decoded before the error are discarded — this is an
/// all-or-nothing operation, matching the JS-side `decodeAll`.
///
/// # Examples
///
/// ```
/// use bijou128::{decode_all, encode};
///
/// let mut buf = Vec::new();
/// for v in [0u128, 42, 500, u128::MAX] {
///     encode(v, &mut buf);
/// }
/// assert_eq!(decode_all(&buf).unwrap(), vec![0, 42, 500, u128::MAX]);
/// ```
pub fn decode_all(buf: &[u8]) -> Result<Vec<u128>, DecodeError> {
    decode_iter(buf).collect()
}

/// Errors that can occur when decoding a `bijou128`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The input buffer is shorter than the encoding requires.
    #[error("buffer too short for bijou128 encoding")]
    BufferTooShort,

    /// The decoded value exceeds `u128::MAX` (tier 16 only).
    #[error("bijou128 tier 16 payload overflows u128")]
    Overflow,
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::needless_range_loop, clippy::panic)]
mod tests {
    use super::*;

    type TestResult = Result<(), DecodeError>;

    mod offset_table {
        use super::*;

        #[test]
        fn recurrence() {
            assert_eq!(OFFSETS[0], 0);
            assert_eq!(OFFSETS[1], u128::from(TAG_THRESHOLD));

            let mut power = 1u128; // 256^0
            for i in 2..=NUM_TIERS {
                power *= 256;
                assert_eq!(
                    OFFSETS[i],
                    OFFSETS[i - 1] + power,
                    "OFFSETS[{i}] does not satisfy recurrence"
                );
            }
        }

        #[test]
        fn known_values() {
            // Pin every tier offset against an *independently* computed
            // closed form — `OFFSETS[t] = TAG_THRESHOLD + sum(256^k for
            // k in 1..t)` — rather than restating the table literals.
            // This catches a typo in `tier_offset`'s recurrence that the
            // `recurrence` test (which uses the same accumulation shape)
            // might share.
            assert_eq!(OFFSETS[0], 0);
            for tier in 1..=NUM_TIERS {
                let mut expected = u128::from(TAG_THRESHOLD);
                let mut power = 1u128; // 256^0
                for _ in 1..tier {
                    power *= 256;
                    expected += power;
                }
                assert_eq!(
                    OFFSETS[tier], expected,
                    "OFFSETS[{tier}] should be {expected}"
                );
            }

            // A few hand-checked anchors for human readability.
            assert_eq!(OFFSETS[1], 240);
            assert_eq!(OFFSETS[2], 240 + 256);
            assert_eq!(OFFSETS[3], 240 + 256 + 65536);
        }

        #[test]
        fn bounds_are_consistent() {
            assert_eq!(BOUNDS[0], OFFSETS[1]);
            for i in 1..NUM_TIERS {
                assert_eq!(
                    BOUNDS[i],
                    OFFSETS[i + 1],
                    "BOUNDS[{i}] should equal OFFSETS[{}]",
                    i + 1
                );
            }
            assert_eq!(BOUNDS[NUM_TIERS], u128::MAX);
        }
    }

    mod round_trip {
        use super::*;

        /// Every tier boundary (min and max) round-trips at the correct length.
        #[test]
        fn all_tier_boundaries() -> TestResult {
            for tier in 1..=NUM_TIERS {
                let min_val = OFFSETS[tier];
                let max_val = if tier < NUM_TIERS {
                    OFFSETS[tier + 1] - 1
                } else {
                    u128::MAX
                };
                let expected_len = 1 + tier;

                for &value in &[min_val, max_val] {
                    let mut buf = Vec::new();
                    encode(value, &mut buf);
                    assert_eq!(
                        buf.len(),
                        expected_len,
                        "tier {tier} value {value}: expected {expected_len} bytes, got {}",
                        buf.len()
                    );

                    let (decoded, consumed) = decode(&buf)?;
                    assert_eq!(decoded, value, "tier {tier} round-trip failed for {value}");
                    assert_eq!(consumed, expected_len);
                }
            }
            Ok(())
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn empty_buffer() {
            assert_eq!(decode(&[]), Err(DecodeError::BufferTooShort));
        }

        #[test]
        fn truncated_at_every_tier() {
            // For each multi-byte tier, provide the tag byte plus one fewer
            // payload byte than required.
            for tier in 1..=NUM_TIERS {
                let tag = u8::try_from(usize::from(TAG_THRESHOLD) + tier - 1).unwrap_or(0xFF);
                let mut buf = vec![tag];
                buf.extend(core::iter::repeat_n(0x00u8, tier - 1));

                assert_eq!(
                    decode(&buf),
                    Err(DecodeError::BufferTooShort),
                    "tier {tier} (tag 0x{tag:02X}) with {}-byte payload \
                     should be BufferTooShort",
                    tier - 1
                );
            }
        }

        #[test]
        fn tier16_overflow() {
            // Tag 0xFF = tier 16, payload = all 0xFF = u128::MAX
            // OFFSETS[16] + u128::MAX overflows.
            let mut buf = vec![0xFF];
            buf.extend(core::iter::repeat_n(0xFFu8, 16));
            assert_eq!(decode(&buf), Err(DecodeError::Overflow));
        }

        #[test]
        fn tier16_overflow_exact_boundary() -> TestResult {
            // The largest valid payload is `u128::MAX - OFFSETS[16]`.
            // One above that overflows.
            let max_payload = u128::MAX - OFFSETS[16];
            let overflow_payload = max_payload + 1;
            let be = overflow_payload.to_be_bytes();
            let mut buf = vec![0xFFu8];
            buf.extend_from_slice(&be);
            assert_eq!(decode(&buf), Err(DecodeError::Overflow));

            // One below: the max valid payload should decode to u128::MAX
            let be_max = max_payload.to_be_bytes();
            let mut buf_max = vec![0xFFu8];
            buf_max.extend_from_slice(&be_max);
            let (value, consumed) = decode(&buf_max)?;
            assert_eq!(value, u128::MAX);
            assert_eq!(consumed, 17);
            Ok(())
        }

        #[test]
        fn trailing_bytes_not_consumed() -> TestResult {
            let (v, n) = decode(&[0x2A, 0xDE, 0xAD])?;
            assert_eq!((v, n), (42, 1));

            let (v, n) = decode(&[0xF0, 0x34, 0xBE, 0xEF])?;
            assert_eq!((v, n), (240 + 0x34, 2));
            Ok(())
        }

        #[test]
        fn decode_always_advances() -> TestResult {
            // For every possible first byte, decode must report `consumed >= 1`.
            // See bijou64's equivalent test for the streaming-consumer rationale.
            let pad = [0u8; MAX_BYTES + 7];
            for first in 0u8..=255u8 {
                let mut buf = Vec::with_capacity(1 + pad.len());
                buf.push(first);
                buf.extend_from_slice(&pad);
                let (_, n) = decode(&buf)?;
                assert!(
                    n >= 1,
                    "decode returned consumed = 0 on first byte {first:#04X}",
                );
            }
            Ok(())
        }
    }

    mod exhaustive {
        use super::*;

        #[test]
        fn tier0() -> TestResult {
            for value in 0..240u128 {
                let mut buf = Vec::new();
                encode(value, &mut buf);
                assert_eq!(buf.len(), 1);
                assert_eq!(buf[0], u8::try_from(value).unwrap_or(0));

                let (decoded, consumed) = decode(&buf)?;
                assert_eq!(decoded, value);
                assert_eq!(consumed, 1);
            }
            Ok(())
        }

        #[test]
        fn tier1() -> TestResult {
            for value in 240..496u128 {
                let mut buf = Vec::new();
                encode(value, &mut buf);
                assert_eq!(buf.len(), 2, "value {value} should encode in 2 bytes");
                assert_eq!(buf[0], 0xF0);

                let (decoded, consumed) = decode(&buf)?;
                assert_eq!(decoded, value, "round-trip failed for {value}");
                assert_eq!(consumed, 2);
            }
            Ok(())
        }

        #[test]
        fn tier2() -> TestResult {
            // Tier 2 spans 496..=66_031 (496 + 256^2 - 1).
            for value in 496..66_032u128 {
                let mut buf = Vec::new();
                encode(value, &mut buf);
                assert_eq!(buf.len(), 3, "value {value} should encode in 3 bytes");
                assert_eq!(buf[0], 0xF1);

                let (decoded, consumed) = decode(&buf)?;
                assert_eq!(decoded, value, "round-trip failed for {value}");
                assert_eq!(consumed, 3);
            }
            Ok(())
        }

        #[test]
        fn tier3_exhaustive() -> TestResult {
            // Tier 3 spans 66_032..=16_843_247 — about 16.8M values, 1–2 s on host.
            for value in 66_032u128..=16_843_247u128 {
                let mut buf = Vec::new();
                encode(value, &mut buf);
                assert_eq!(buf.len(), 4, "value {value} should encode in 4 bytes");
                assert_eq!(buf[0], 0xF2);

                let (decoded, consumed) = decode(&buf)?;
                assert_eq!(decoded, value, "round-trip failed for {value}");
                assert_eq!(consumed, 4);
            }
            Ok(())
        }

        #[test]
        fn canonicality_byte_sequence_exhaustive() -> TestResult {
            let check = |buf: &[u8]| -> TestResult {
                // `Err(_)` (truncated / overflow) is fine here — we
                // only need to check canonicality on successful decodes.
                if let Ok((value, consumed)) = decode(buf) {
                    let mut re = Vec::with_capacity(MAX_BYTES);
                    encode(value, &mut re);
                    assert_eq!(
                        re.as_slice(),
                        &buf[..consumed],
                        "non-canonical: decode({:02X?}) = {value}, re-encode = {:02X?}",
                        &buf[..consumed],
                        re
                    );
                }
                Ok(())
            };

            // Tier 0 + tier 1 + tier 2 (256 + 256 + 65,536 = 66,048 cases).
            for b in 0u8..=255u8 {
                check(&[b])?;
            }
            for p in 0u8..=255u8 {
                check(&[0xF0, p])?;
            }
            for p1 in 0u8..=255u8 {
                for p2 in 0u8..=255u8 {
                    check(&[0xF1, p1, p2])?;
                }
            }
            // Tier 3: 16,777,216 cases.
            for p in 0u32..(1u32 << 24) {
                let bytes = p.to_be_bytes();
                check(&[0xF2, bytes[1], bytes[2], bytes[3]])?;
            }
            Ok(())
        }
    }

    mod boundaries {
        use super::*;

        #[test]
        fn offset_triples() -> TestResult {
            // For each tier boundary OFFSET[n], verify that:
            //   OFFSET[n] - 1  encodes in tier n-1 (shorter)
            //   OFFSET[n]      encodes in tier n
            //   OFFSET[n] + 1  encodes in tier n (same length)
            for tier in 1..=NUM_TIERS {
                let offset = OFFSETS[tier];
                let tier_len = 1 + tier;
                let prev_len = if tier == 1 { 1 } else { 1 + (tier - 1) };

                // OFFSET[n] - 1: last value of the previous tier
                let below = offset - 1;
                let mut buf = Vec::new();
                encode(below, &mut buf);
                assert_eq!(
                    buf.len(),
                    prev_len,
                    "OFFSETS[{tier}] - 1 = {below}: expected {prev_len} bytes, got {}",
                    buf.len()
                );
                let (v, n) = decode(&buf)?;
                assert_eq!(v, below);
                assert_eq!(n, prev_len);

                // OFFSET[n]: first value of this tier
                buf.clear();
                encode(offset, &mut buf);
                assert_eq!(
                    buf.len(),
                    tier_len,
                    "OFFSETS[{tier}] = {offset}: expected {tier_len} bytes, got {}",
                    buf.len()
                );
                let (v, n) = decode(&buf)?;
                assert_eq!(v, offset);
                assert_eq!(n, tier_len);

                // OFFSET[n] + 1: second value of this tier
                if offset < u128::MAX {
                    buf.clear();
                    encode(offset + 1, &mut buf);
                    assert_eq!(
                        buf.len(),
                        tier_len,
                        "OFFSETS[{tier}] + 1 = {}: expected {tier_len} bytes, got {}",
                        offset + 1,
                        buf.len()
                    );
                    let (v, n) = decode(&buf)?;
                    assert_eq!(v, offset + 1);
                    assert_eq!(n, tier_len);
                }
            }
            Ok(())
        }

        #[test]
        fn all_zero_payloads() -> TestResult {
            // [tag, 0x00, ..., 0x00] should decode to OFFSETS[tier] for each tier.
            for tier in 1..=NUM_TIERS {
                let tag = u8::try_from(usize::from(TAG_THRESHOLD) + tier - 1).unwrap_or(0xFF);
                let mut buf = vec![tag];
                buf.extend(core::iter::repeat_n(0x00u8, tier));

                let (value, consumed) = decode(&buf)?;
                assert_eq!(
                    value, OFFSETS[tier],
                    "tier {tier} all-zeros payload: expected OFFSETS[{tier}] = {}, got {value}",
                    OFFSETS[tier]
                );
                assert_eq!(consumed, 1 + tier);

                let mut re = Vec::new();
                encode(value, &mut re);
                assert_eq!(re, buf, "tier {tier} all-zeros round-trip mismatch");
            }
            Ok(())
        }

        #[test]
        fn all_ones_payloads() -> TestResult {
            // [tag, 0xFF, ..., 0xFF] should decode to the tier's maximum value
            for tier in 1..=NUM_TIERS {
                let tag = u8::try_from(usize::from(TAG_THRESHOLD) + tier - 1).unwrap_or(0xFF);
                let mut buf = vec![tag];
                buf.extend(core::iter::repeat_n(0xFFu8, tier));

                let result = decode(&buf);

                if tier < NUM_TIERS {
                    // Tiers 1–15: all-ones payload = 256^tier - 1, so
                    // value = OFFSETS[tier] + (256^tier - 1) = OFFSETS[tier+1] - 1
                    let (value, consumed) = result?;
                    let expected = OFFSETS[tier + 1] - 1;
                    assert_eq!(
                        value, expected,
                        "tier {tier} all-ones payload: expected {expected}, got {value}"
                    );
                    assert_eq!(consumed, 1 + tier);

                    let mut re = Vec::new();
                    encode(value, &mut re);
                    assert_eq!(re, buf, "tier {tier} all-ones round-trip mismatch");
                } else {
                    // Tier 16: all-ones payload = u128::MAX, OFFSETS[16] + u128::MAX overflows
                    assert_eq!(
                        result,
                        Err(DecodeError::Overflow),
                        "tier 16 all-ones should overflow"
                    );
                }
            }
            Ok(())
        }
    }

    mod bijectivity {
        use super::*;

        #[test]
        fn overlong_encoding_decodes_to_different_value() -> TestResult {
            // For each tier 1..NUM_TIERS-1, take the first value of that tier
            // and manually encode it in the *next* tier's format with the same
            // numeric payload (without re-adding the offset). The decoder adds
            // OFFSETS[tier+1] instead of OFFSETS[tier], so the result must differ.
            for tier in 1..NUM_TIERS {
                let value = OFFSETS[tier];
                let payload = value - OFFSETS[tier]; // == 0

                let wider_tier = tier + 1;
                let tag = u8::try_from(usize::from(TAG_THRESHOLD) + wider_tier - 1).unwrap_or(0xFF);
                let mut forged = vec![tag];
                let be = payload.to_be_bytes();
                forged.extend_from_slice(be.get(16 - wider_tier..).unwrap_or(&[]));

                let (decoded, _) = decode(&forged)?;

                assert_ne!(
                    decoded, value,
                    "tier {tier}: overlong encoding of {value} decoded back \
                     to {value} — bijectivity violated"
                );
                assert_eq!(
                    decoded, OFFSETS[wider_tier],
                    "tier {tier}: forged payload 0 in tier {wider_tier} should \
                     decode to OFFSETS[{wider_tier}]"
                );
            }
            Ok(())
        }
    }

    mod streaming {
        use super::*;

        #[test]
        fn consecutive_decode() -> TestResult {
            let values: &[(u128, u128)] = &[
                (0, 0),
                (42, 500),
                (240, 496),
                (1u128 << 64, u128::MAX),
                (u128::MAX, 0),
            ];

            for &(a, b) in values {
                let mut buf = Vec::new();
                encode(a, &mut buf);
                encode(b, &mut buf);

                let (decoded_a, consumed_a) = decode(&buf)?;
                assert_eq!(decoded_a, a, "first value mismatch for ({a}, {b})");

                let (decoded_b, consumed_b) = decode(&buf[consumed_a..])?;
                assert_eq!(decoded_b, b, "second value mismatch for ({a}, {b})");
                assert_eq!(
                    consumed_a + consumed_b,
                    buf.len(),
                    "total consumed mismatch for ({a}, {b})"
                );
            }
            Ok(())
        }
    }

    mod encoded_bytes {
        use super::*;

        #[test]
        fn agrees_with_encode() {
            let probes: &[u128] = &[
                0,
                1,
                239,
                240,
                495,
                496,
                65_535,
                66_031,
                66_032,
                1u128 << 32,
                1u128 << 64,
                1u128 << 96,
                u128::MAX,
            ];
            for &v in probes {
                let enc = encoded_bytes(v);
                let mut via_encode = Vec::new();
                encode(v, &mut via_encode);
                assert_eq!(enc.len(), via_encode.len(), "len mismatch for {v}");
                assert_eq!(&*enc, via_encode.as_slice(), "bytes mismatch for {v}");
            }
        }

        #[test]
        fn deref_asref_as_slice_consistent() {
            let enc = encoded_bytes(240);
            let via_deref: &[u8] = &enc;
            let via_asref: &[u8] = enc.as_ref();
            let via_as_slice: &[u8] = enc.as_slice();
            assert_eq!(via_deref, &[0xF0, 0x00]);
            assert_eq!(via_asref, &[0xF0, 0x00]);
            assert_eq!(via_as_slice, &[0xF0, 0x00]);
        }

        #[test]
        fn into_iter_walks_only_encoded_bytes() {
            let enc = encoded_bytes(240); // 2 bytes
            let collected: Vec<u8> = enc.into_iter().collect();
            assert_eq!(collected, [0xF0, 0x00]);
        }

        #[test]
        fn ref_into_iter_yields_encoded_bytes() {
            let enc = encoded_bytes(240);
            let collected: Vec<u8> = (&enc).into_iter().copied().collect();
            assert_eq!(collected, [0xF0, 0x00]);
        }

        #[test]
        fn copy_semantics() {
            let enc = encoded_bytes(42);
            let dup = enc;
            assert_eq!(&*enc, &[0x2A]);
            assert_eq!(&*dup, &[0x2A]);
        }

        #[test]
        fn is_empty_is_always_false() {
            for v in [0u128, 1, 239, 240, u128::MAX] {
                assert!(!encoded_bytes(v).is_empty(), "is_empty true for {v}");
            }
        }

        #[test]
        fn borrow_impl() {
            use alloc::collections::BTreeMap;
            let mut map: BTreeMap<EncodedBytes, &'static str> = BTreeMap::new();
            map.insert(encoded_bytes(42), "the answer");
            assert_eq!(map.get(&[0x2A][..]), Some(&"the answer"));
        }

        #[test]
        fn eq_partial_eq() {
            let a = encoded_bytes(500);
            let b = encoded_bytes(500);
            let c = encoded_bytes(501);
            assert_eq!(a, b);
            assert_ne!(a, c);
        }

        #[test]
        fn round_trip_via_decode() -> TestResult {
            for v in [0u128, 239, 240, 65_535, 1u128 << 64, u128::MAX] {
                let enc = encoded_bytes(v);
                let (decoded, n) = decode(&enc)?;
                assert_eq!(decoded, v);
                assert_eq!(n, enc.len());
            }
            Ok(())
        }
    }

    mod iter {
        use super::*;

        #[test]
        fn empty_buffer() {
            let collected: Vec<_> = decode_iter(&[]).collect();
            assert!(collected.is_empty());
        }

        #[test]
        fn round_trip() -> TestResult {
            let values: &[u128] = &[
                0,
                1,
                42,
                239,
                240,
                495,
                496,
                65_535,
                1u128 << 64,
                1u128 << 96,
                u128::MAX,
            ];
            let mut buf = Vec::new();
            for &v in values {
                encode(v, &mut buf);
            }
            let collected: Result<Vec<u128>, _> = decode_iter(&buf).collect();
            assert_eq!(collected?, values);
            Ok(())
        }

        #[test]
        fn fuses_after_error() {
            // Tag 0xF0 with no payload → BufferTooShort.
            let mut iter = decode_iter(&[0xF0]);
            assert_eq!(iter.next(), Some(Err(DecodeError::BufferTooShort)));
            assert_eq!(iter.next(), None, "iter must fuse after error");
            assert_eq!(iter.next(), None, "iter must remain fused");
        }

        #[test]
        fn overflow_error_then_fused() {
            let buf = [0xFFu8; 17];
            let mut iter = decode_iter(&buf);
            assert_eq!(iter.next(), Some(Err(DecodeError::Overflow)));
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn partial_success_then_error() {
            let mut iter = decode_iter(&[0x42, 0xF0]);
            assert_eq!(iter.next(), Some(Ok(0x42)));
            assert_eq!(iter.next(), Some(Err(DecodeError::BufferTooShort)));
            assert_eq!(iter.next(), None);
        }

        #[test]
        fn size_hint() {
            let empty = decode_iter(&[]);
            assert_eq!(empty.size_hint(), (0, Some(0)));

            let with_data = decode_iter(&[0x42, 0x99]);
            assert_eq!(with_data.size_hint(), (1, Some(2)));

            let mut errored = decode_iter(&[0xF0]);
            let _ = errored.next();
            assert_eq!(errored.size_hint(), (0, Some(0)));
        }

        #[test]
        fn composable_with_combinators() {
            let mut buf = Vec::new();
            for v in [10u128, 20, 30, 40, 50] {
                encode(v, &mut buf);
            }
            let sum: u128 = decode_iter(&buf).filter_map(Result::ok).sum();
            assert_eq!(sum, 150);

            let first_two: Vec<_> = decode_iter(&buf).take(2).filter_map(Result::ok).collect();
            assert_eq!(first_two, [10, 20]);
        }
    }

    mod decode_all {
        use super::*;

        #[test]
        fn empty_buffer() -> TestResult {
            assert_eq!(decode_all(&[])?, Vec::<u128>::new());
            Ok(())
        }

        #[test]
        fn round_trip() -> TestResult {
            let values: &[u128] = &[0, 42, 500, 65_535, 1u128 << 64, 1u128 << 96, u128::MAX];
            let mut buf = Vec::new();
            for &v in values {
                encode(v, &mut buf);
            }
            assert_eq!(decode_all(&buf)?, values);
            Ok(())
        }

        #[test]
        fn short_circuits_on_first_error() {
            assert_eq!(decode_all(&[0x42, 0xF0]), Err(DecodeError::BufferTooShort));
        }

        #[test]
        fn overflow_propagates() {
            assert_eq!(decode_all(&[0xFF; 17]), Err(DecodeError::Overflow));
        }

        #[test]
        fn agrees_with_decode_iter_collect() {
            let inputs: &[&[u8]] = &[&[], &[0x00], &[0x42, 0xF0, 0x34], &[0xF0]];
            for &input in inputs {
                let via_iter: Result<Vec<u128>, _> = decode_iter(input).collect();
                let via_all = decode_all(input);
                assert_eq!(via_iter, via_all, "disagreement on {input:02X?}");
            }
        }
    }

    mod encode_api {
        use super::*;

        #[test]
        fn appends_to_non_empty_buffer() -> TestResult {
            let mut buf = vec![0xDE, 0xAD];
            encode(500, &mut buf);

            assert_eq!(&buf[..2], &[0xDE, 0xAD]);

            let (value, consumed) = decode(&buf[2..])?;
            assert_eq!(value, 500);
            assert_eq!(consumed, 3);
            assert_eq!(buf.len(), 5);
            Ok(())
        }

        /// `MAX_BYTES` must equal the actual worst-case encoded length.
        /// `u128::MAX` is the worst case, so this single input fully covers
        /// the invariant. Guards against a future tier-layout change that
        /// updates `MAX_BYTES` but not the encoder (or vice versa).
        #[test]
        fn max_bytes_equals_encoded_len_of_max() {
            let mut buf = Vec::new();
            encode(u128::MAX, &mut buf);
            assert_eq!(
                buf.len(),
                MAX_BYTES,
                "MAX_BYTES disagrees with encode(u128::MAX).len()"
            );
            assert_eq!(encoded_len(u128::MAX), MAX_BYTES);
            assert_eq!(encoded_bytes(u128::MAX).len(), MAX_BYTES);
        }
    }

    mod test_vectors {
        use super::*;

        /// Hand-verified test vectors. Replicate these in any second
        /// implementation to verify encoding compatibility.
        const VECTORS: &[(u128, &[u8])] = &[
            // Tier 0: single byte
            (0, &[0x00]),
            (1, &[0x01]),
            (42, &[0x2A]),
            (239, &[0xEF]),
            // Tier 1: tag 0xF0 + 1 byte
            (240, &[0xF0, 0x00]),
            (241, &[0xF0, 0x01]),
            (495, &[0xF0, 0xFF]),
            // Tier 2: tag 0xF1 + 2 bytes
            (496, &[0xF1, 0x00, 0x00]),
            (65_535, &[0xF1, 0xFE, 0x0F]),
            (66_031, &[0xF1, 0xFF, 0xFF]),
            // Tier 3: tag 0xF2 + 3 bytes
            (66_032, &[0xF2, 0x00, 0x00, 0x00]),
            (67_000, &[0xF2, 0x00, 0x03, 0xC8]),
            (16_843_247, &[0xF2, 0xFF, 0xFF, 0xFF]),
            // Tier 4: tag 0xF3 + 4 bytes
            (16_843_248, &[0xF3, 0x00, 0x00, 0x00, 0x00]),
            ((1u128 << 32) - 1, &[0xF3, 0xFE, 0xFE, 0xFE, 0x0F]),
            // Tier 8: tag 0xF7 + 8 bytes; representative for the
            // u64 boundary (`(1 << 64) - 1`).
            (
                (1u128 << 64) - 1,
                &[0xF7, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F],
            ),
            // Tier 16: tag 0xFF + 16 bytes (u128::MAX)
            (
                u128::MAX,
                &[
                    0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
                    0xFE, 0xFE, 0xFE, 0x0F,
                ],
            ),
        ];

        #[test]
        fn encode_vectors() {
            for &(value, expected) in VECTORS {
                let mut buf = Vec::new();
                super::encode(value, &mut buf);
                assert_eq!(
                    buf.as_slice(),
                    expected,
                    "encode({value}) mismatch: got {buf:02X?}, expected {expected:02X?}"
                );
            }
        }

        #[test]
        fn decode_vectors() -> TestResult {
            for &(expected_value, bytes) in VECTORS {
                let (value, consumed) = super::decode(bytes)?;
                assert_eq!(
                    value, expected_value,
                    "decode({bytes:02X?}): got {value}, expected {expected_value}"
                );
                assert_eq!(consumed, bytes.len());
            }
            Ok(())
        }
    }

    #[cfg(feature = "bolero")]
    mod property {
        use super::*;

        #[test]
        #[cfg_attr(miri, ignore)]
        fn round_trip() {
            bolero::check!()
                .with_arbitrary::<u128>()
                .for_each(|&value| {
                    let mut buf = Vec::new();
                    encode(value, &mut buf);
                    let (decoded, consumed) = decode(&buf).unwrap_or_else(|e| {
                        panic!("round-trip decode failed for {value}: {e}");
                    });
                    assert_eq!(decoded, value, "round-trip failed for {value}");
                    assert_eq!(consumed, buf.len());
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn encoded_len_matches() {
            bolero::check!()
                .with_arbitrary::<u128>()
                .for_each(|&value| {
                    let mut buf = Vec::new();
                    encode(value, &mut buf);
                    assert_eq!(encoded_len(value), buf.len());
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn encoded_bytes_matches() {
            bolero::check!()
                .with_arbitrary::<u128>()
                .for_each(|&value| {
                    let mut buf = Vec::new();
                    encode(value, &mut buf);
                    let enc = encoded_bytes(value);
                    assert_eq!(&*enc, buf.as_slice(), "value {value}");
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn decode_never_panics() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|buf| {
                    let _ = decode(buf);
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn decode_always_advances() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|buf| {
                    if let Ok((_, consumed)) = decode(buf) {
                        assert!(consumed >= 1, "decode of {buf:02X?} returned consumed = 0");
                    }
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn lexicographic_order() {
            bolero::check!()
                .with_arbitrary::<(u128, u128)>()
                .for_each(|&(a, b)| {
                    let enc_a = encoded_bytes(a);
                    let enc_b = encoded_bytes(b);
                    let slice_a: &[u8] = &enc_a;
                    let slice_b: &[u8] = &enc_b;
                    assert_eq!(
                        a.cmp(&b),
                        slice_a.cmp(slice_b),
                        "order mismatch: {a} vs {b}, \
                         encoded {slice_a:02X?} vs {slice_b:02X?}",
                    );
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn bijective() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|buf| {
                    if let Ok((value, consumed)) = decode(buf) {
                        let mut re_encoded = Vec::new();
                        encode(value, &mut re_encoded);
                        assert_eq!(
                            re_encoded.as_slice(),
                            buf.get(..consumed).unwrap_or_default(),
                            "bijection violated: decode({:02X?}) = {value}, \
                             re-encode = {:02X?}",
                            buf.get(..consumed).unwrap_or_default(),
                            re_encoded
                        );
                    }
                });
        }

        /// Whole-stream roundtrip: a packed stream of N arbitrary values
        /// decodes back to exactly those N values.
        ///
        /// `forall xs. decode_all(concat(map(encode, xs))) == Ok(xs)`
        #[test]
        #[cfg_attr(miri, ignore)]
        fn decode_all_roundtrips_arbitrary_streams() {
            bolero::check!()
                .with_arbitrary::<Vec<u128>>()
                .for_each(|xs| {
                    let mut buf = Vec::new();
                    for &x in xs {
                        encode(x, &mut buf);
                    }
                    assert_eq!(decode_all(&buf).as_deref(), Ok(xs.as_slice()));
                });
        }

        /// Oracle: `decode_iter` must behave exactly like the hand-rolled
        /// cursor loop it replaces, on arbitrary (possibly malformed)
        /// bytes — same successful prefix, same first error, then stop.
        #[test]
        #[cfg_attr(miri, ignore)]
        fn decode_iter_matches_manual_cursor() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|buf| {
                    let via_iter: Vec<Result<u128, DecodeError>> = decode_iter(buf).collect();

                    let mut manual = Vec::new();
                    let mut cursor: &[u8] = buf;
                    loop {
                        if cursor.is_empty() {
                            break;
                        }
                        match decode(cursor) {
                            Ok((v, n)) => {
                                manual.push(Ok(v));
                                cursor = cursor.get(n..).unwrap_or_default();
                            }
                            Err(e) => {
                                manual.push(Err(e));
                                break; // iterator fuses on error
                            }
                        }
                    }
                    assert_eq!(
                        via_iter, manual,
                        "decode_iter disagreed with manual loop on {buf:02X?}"
                    );
                });
        }

        /// Once `decode_iter` yields an `Err`, every subsequent `next()`
        /// must be `None` (the iterator is fused).
        #[test]
        #[cfg_attr(miri, ignore)]
        fn decode_iter_fuses_after_any_error() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|buf| {
                    let mut it = decode_iter(buf);
                    let mut seen_err = false;
                    for item in it.by_ref() {
                        assert!(
                            !seen_err,
                            "decode_iter yielded after an error on {buf:02X?}"
                        );
                        if item.is_err() {
                            seen_err = true;
                        }
                    }
                    if seen_err {
                        assert!(it.next().is_none(), "must stay fused");
                        assert!(it.next().is_none(), "must stay fused");
                    }
                });
        }
    }
}
