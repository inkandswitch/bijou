//! Bijective variable-length encoding for unsigned 128-bit integers.
//!
//! bijou128 (**BIJ**ective **O**ffset **U128**) encodes `u128` values into
//! 1–17 bytes using a tag-byte prefix scheme derived from [VARU64], modified
//! with per-tier offsets to achieve **structural canonicality** — each value
//! has exactly one encoding, and each encoding has exactly one value. This is
//! [bijective numeration] applied to VARU64-style tag-byte framing.
//!
//! See [`bijou64`] for the 64-bit variant. bijou128 uses the same family of
//! tricks with two structural differences:
//!
//! - Tag threshold is **240** (vs 248 in bijou64), giving 16 multi-byte
//!   tiers in tag bytes `0xF0..=0xFF` (vs 8 tiers in `0xF8..=0xFF`).
//! - Tier 0 (single-byte values) covers `0..=239` (vs `0..=247`).
//!
//! These shifts let all 16 tiers fit in a single tag byte, preserving
//! length-from-first-byte behaviour. No extended framing is required.
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
//! ┌───────────┬──────────────────┬──────────────────────────────┐
//! │ Tag       │ Additional bytes │ Value range                  │
//! ├───────────┼──────────────────┼──────────────────────────────┤
//! │ 0x00–0xEF │ 0                │ 0 – 239                      │
//! │ 0xF0      │ 1                │ 240 – 495                    │
//! │ 0xF1      │ 2                │ 496 – 66,031                 │
//! │ 0xF2      │ 3                │ 66,032 – 16,843,247          │
//! │ 0xF3      │ 4                │ 16,843,248 – 4,311,810,543   │
//! │ ...       │ ...              │ ...                          │
//! │ 0xFF      │ 16               │ OFFSETS[16] – u128::MAX      │
//! └───────────┴──────────────────┴──────────────────────────────┘
//! ```
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
//! let mut buf = Vec::new();
//! bijou128::encode(300, &mut buf);
//! assert_eq!(buf, [0xF0, 0x3C]); // tag 240, payload 300 - 240 = 60
//!
//! let (value, len) = bijou128::decode(&buf).unwrap();
//! assert_eq!(value, 300);
//! assert_eq!(len, 2);
//! ```
//!
//! [VARU64]: https://github.com/AljoschaMeyer/varu64-rs
//! [`bijou64`]: https://crates.io/crates/bijou64
//! [bijective numeration]: https://en.wikipedia.org/wiki/Bijective_numeration

#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

#[allow(unused_imports)] // vec! macro used in tests
use alloc::{vec, vec::Vec};

/// Maximum number of bytes a `bijou128` encoding can occupy.
pub const MAX_BYTES: usize = 17;

/// Tag byte threshold: values below this are encoded as a single byte.
const TAG_THRESHOLD: u8 = 240;

/// Number of multi-byte tiers (tags 240–255 → tiers 1–16).
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

/// Per-tier upper bounds.
///
/// A value belongs to tier `t` if `OFFSETS[t] <= value < BOUNDS[t]` for
/// `t` in `0..NUM_TIERS`. `BOUNDS[t] == OFFSETS[t + 1]` in that range,
/// so the bound is exclusive.
///
/// For the final tier (`t == NUM_TIERS == 16`), `BOUNDS[16] == u128::MAX`
/// and the bound is **inclusive** — the comparison degenerates to
/// `OFFSETS[16] <= value <= u128::MAX`. The decoder handles arithmetic
/// overflow via `checked_add` rather than the bound check.
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
/// offsets.
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

    // Tier boundaries align to bit-widths 8, 9, 17, 25, ..., 121.
    // For bw=8 -> candidate 2, bw 9..=16 -> 3, 17..=24 -> 4, etc.
    // Formula: (bw - 1) / 8 + 2.
    let candidate = ((bw - 1) / 8 + 2) as usize;

    // The candidate can be at most 1 too high because bijou128's tier
    // offsets push the boundary slightly past each power-of-256.
    // One comparison corrects for boundary values.
    //
    // SAFETY (indexing): candidate ∈ [2, 17] because bw ∈ [8, 128] after
    // the tier-0 early return, so candidate - 2 ∈ [0, 15] — always in
    // bounds for the 17-element BOUNDS array.
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

    let tag = ((TAG_THRESHOLD as usize - 1) + tier) as u8;
    let payload = (value - OFFSETS[tier]) << (8 * (16 - tier));
    let pb = payload.to_be_bytes();

    let original_len = buf.len();
    buf.extend_from_slice(&[
        tag, pb[0], pb[1], pb[2], pb[3], pb[4], pb[5], pb[6], pb[7], pb[8], pb[9], pb[10], pb[11],
        pb[12], pb[13], pb[14], pb[15],
    ]);
    buf.truncate(original_len + tier + 1);
}

/// Encodes `value` as a `bijou128` into a fixed-size array.
///
/// Returns `(bytes, len)` where `bytes` is a 17-byte array with the
/// encoding in `bytes[..len]`.
///
/// Uses `leading_zeros` to derive the tier in O(1) rather than walking
/// an if/else chain. See [`encoded_len`] for the same technique.
///
/// # Examples
///
/// ```
/// let (bytes, len) = bijou128::encode_array(300);
/// assert_eq!(&bytes[..len], &[0xF0, 0x3C]);
/// ```
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::indexing_slicing)]
pub const fn encode_array(value: u128) -> ([u8; MAX_BYTES], usize) {
    if value < BOUNDS[0] {
        return (
            [
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
            1,
        );
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
    let tag = ((TAG_THRESHOLD as usize - 1) + tier) as u8;
    let payload = (value - OFFSETS[tier]) << (8 * (16 - tier));
    let pb = payload.to_be_bytes();

    (
        [
            tag, pb[0], pb[1], pb[2], pb[3], pb[4], pb[5], pb[6], pb[7], pb[8], pb[9], pb[10],
            pb[11], pb[12], pb[13], pb[14], pb[15],
        ],
        tier + 1,
    )
}

/// Decodes a `bijou128` from the front of `buf`.
///
/// Returns `(value, bytes_consumed)` on success.
///
/// # Errors
///
/// - [`DecodeError::BufferTooShort`] if `buf` has fewer bytes than the
///   encoding requires.
/// - [`DecodeError::Overflow`] if a tier 16 payload, when added to
///   `OFFSETS[16]`, exceeds `u128::MAX`.
///
/// # Examples
///
/// ```
/// // Single-byte value
/// let (v, n) = bijou128::decode(&[0x2A]).unwrap();
/// assert_eq!((v, n), (42, 1));
///
/// // Multi-byte value with trailing data
/// let (v, n) = bijou128::decode(&[0xF0, 0x3C, 0xFF]).unwrap();
/// assert_eq!((v, n), (300, 2));
/// ```
#[inline]
#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // 16-arm match
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
    // 16 arms, one per tag byte 0xF0..=0xFF. Each arm specialises to a
    // fixed-size copy via LLVM. Do not consolidate — bijou64 measurements
    // showed a tier-dispatched while-loop variant runs 4–7× slower.
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
            assert_eq!(OFFSETS[1], 240);

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
            assert_eq!(OFFSETS[1], 0xF0);
            assert_eq!(OFFSETS[2], 0x1F0);
            assert_eq!(OFFSETS[3], 0x1_01F0);
            assert_eq!(OFFSETS[4], 0x101_01F0);
            assert_eq!(OFFSETS[5], 0x1_0101_01F0);
            assert_eq!(OFFSETS[6], 0x101_0101_01F0);
            assert_eq!(OFFSETS[7], 0x1_0101_0101_01F0);
            assert_eq!(OFFSETS[8], 0x101_0101_0101_01F0);
            assert_eq!(OFFSETS[9], 0x1_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[10], 0x101_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[11], 0x1_0101_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[12], 0x101_0101_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[13], 0x1_0101_0101_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[14], 0x101_0101_0101_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[15], 0x1_0101_0101_0101_0101_0101_0101_01F0);
            assert_eq!(OFFSETS[16], 0x101_0101_0101_0101_0101_0101_0101_01F0);
        }

        #[test]
        fn bounds_are_consistent() {
            assert_eq!(BOUNDS[0], 240);
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
                let tag = u8::try_from(TAG_THRESHOLD as usize - 1 + tier).unwrap_or(0xFF);
                let mut buf = vec![tag];
                // tier needs `tier` payload bytes; provide `tier - 1`
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
            // OFFSETS[16] + u128::MAX overflows
            let buf = [0xFFu8; 17];
            assert_eq!(decode(&buf), Err(DecodeError::Overflow));
        }

        #[test]
        fn tier16_overflow_exact_boundary() -> TestResult {
            // Smallest tier 16 payload that overflows: u128::MAX - OFFSETS[16] + 1
            let max_payload = u128::MAX - OFFSETS[16];
            let overflow_payload = max_payload + 1;
            let be = overflow_payload.to_be_bytes();
            let mut buf = [0u8; 17];
            buf[0] = 0xFF;
            buf[1..].copy_from_slice(&be);
            assert_eq!(decode(&buf), Err(DecodeError::Overflow));

            // One below: the max valid payload should decode to u128::MAX
            let be_max = max_payload.to_be_bytes();
            let mut buf_max = [0u8; 17];
            buf_max[0] = 0xFF;
            buf_max[1..].copy_from_slice(&be_max);
            let (value, consumed) = decode(&buf_max)?;
            assert_eq!(value, u128::MAX);
            assert_eq!(consumed, 17);
            Ok(())
        }

        #[test]
        fn trailing_bytes_not_consumed() -> TestResult {
            let (v, n) = decode(&[0x2A, 0xDE, 0xAD])?;
            assert_eq!((v, n), (42, 1));

            let (v, n) = decode(&[0xF0, 0x3C, 0xBE, 0xEF])?;
            assert_eq!((v, n), (300, 2));
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
            // [tag, 0x00, ..., 0x00] should decode to OFFSETS[tier] for each tier
            for tier in 1..=NUM_TIERS {
                let tag = u8::try_from(TAG_THRESHOLD as usize - 1 + tier).unwrap_or(0xFF);
                let mut buf = vec![tag];
                buf.extend(core::iter::repeat_n(0x00u8, tier));

                let (value, consumed) = decode(&buf)?;
                assert_eq!(
                    value, OFFSETS[tier],
                    "tier {tier} all-zeros payload: expected OFFSETS[{tier}] = {}, got {value}",
                    OFFSETS[tier]
                );
                assert_eq!(consumed, 1 + tier);

                // Round-trip: re-encoding should produce the same bytes
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
                let tag = u8::try_from(TAG_THRESHOLD as usize - 1 + tier).unwrap_or(0xFF);
                let mut buf = vec![tag];
                buf.extend(core::iter::repeat_n(0xFFu8, tier));

                let result = decode(&buf);

                if tier < NUM_TIERS {
                    // Tiers 1-15: all-ones payload = 256^tier - 1, so
                    // value = OFFSETS[tier] + (256^tier - 1) = OFFSETS[tier+1] - 1
                    let (value, consumed) = result?;
                    let expected = OFFSETS[tier + 1] - 1;
                    assert_eq!(
                        value, expected,
                        "tier {tier} all-ones payload: expected {expected}, got {value}"
                    );
                    assert_eq!(consumed, 1 + tier);

                    // Round-trip
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
            // For each tier 1..15, take a value that belongs to that tier and
            // manually encode it in the *next* tier's format (wider tag, same
            // numeric payload without re-adding the offset). Because the
            // decoder adds OFFSETS[tier+1] instead of OFFSETS[tier], the
            // decoded value must differ — the offset shift structurally
            // prevents overlong encodings from round-tripping.
            for tier in 1..NUM_TIERS {
                let value = OFFSETS[tier]; // first value in this tier
                let payload = value - OFFSETS[tier]; // == 0

                // Forge: use next tier's tag with the same payload bytes
                let wider_tier = tier + 1;
                let tag = u8::try_from(TAG_THRESHOLD as usize - 1 + wider_tier).unwrap_or(0xFF);
                let mut forged = vec![tag];
                let be = payload.to_be_bytes();
                forged.extend_from_slice(be.get(16 - wider_tier..).unwrap_or(&[]));

                let (decoded, _) = decode(&forged)?;

                // The forged encoding should decode to OFFSETS[wider_tier],
                // not the original value (unless they happen to be equal,
                // which they never are since OFFSETS is strictly increasing).
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
            // Encode two values back-to-back into a single buffer, then
            // decode them sequentially using the returned `consumed` offset.
            let values: &[(u128, u128)] = &[
                (0, 0),
                (42, 300),
                (240, 496),
                (0x1_01F0, u128::MAX),
                (u128::MAX, 0),
                (u128::from(u64::MAX), u128::MAX),
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

    mod encode_api {
        use super::*;

        #[test]
        fn appends_to_non_empty_buffer() -> TestResult {
            let mut buf = vec![0xDE, 0xAD];
            encode(300, &mut buf);

            // Original bytes preserved
            assert_eq!(&buf[..2], &[0xDE, 0xAD]);

            // Appended encoding is correct
            let (value, consumed) = decode(&buf[2..])?;
            assert_eq!(value, 300);
            assert_eq!(consumed, 2);
            assert_eq!(buf.len(), 4); // 2 prefix + 2 encoding
            Ok(())
        }
    }

    mod test_vectors {
        use super::*;

        /// Test vectors: `(value, expected_bytes)`.
        ///
        /// These vectors should be replicated in any second implementation
        /// (e.g., TypeScript) to verify encoding compatibility.
        const VECTORS: &[(u128, &[u8])] = &[
            // Tier 0: single byte
            (0, &[0x00]),
            (1, &[0x01]),
            (42, &[0x2A]),
            (97, &[0x61]),
            (127, &[0x7F]),
            (128, &[0x80]),
            (200, &[0xC8]),
            (239, &[0xEF]),
            // Tier 1: tag 0xF0 + 1 byte (240..=495)
            (240, &[0xF0, 0x00]),
            (241, &[0xF0, 0x01]),
            (300, &[0xF0, 0x3C]),
            (495, &[0xF0, 0xFF]),
            // Tier 2: tag 0xF1 + 2 bytes (496..=66_031)
            (496, &[0xF1, 0x00, 0x00]),
            (1_000, &[0xF1, 0x01, 0xF8]),
            (65_535, &[0xF1, 0xFE, 0x0F]),
            (66_031, &[0xF1, 0xFF, 0xFF]),
            // Tier 3: tag 0xF2 + 3 bytes (66_032..=16_843_247)
            (0x1_01F0, &[0xF2, 0x00, 0x00, 0x00]),
            (0x101_01EF, &[0xF2, 0xFF, 0xFF, 0xFF]),
            // Tier 4: tag 0xF3 + 4 bytes
            (0x101_01F0, &[0xF3, 0x00, 0x00, 0x00, 0x00]),
            (0x1_0101_01EF, &[0xF3, 0xFF, 0xFF, 0xFF, 0xFF]),
            // Tier 8: tag 0xF7 + 8 bytes
            (
                0x101_0101_0101_01F0,
                &[0xF7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            ),
            // Tier 16: tag 0xFF + 16 bytes
            (
                0x101_0101_0101_0101_0101_0101_0101_01F0,
                &[
                    0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00,
                ],
            ),
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
        fn encode_array_matches() {
            bolero::check!()
                .with_arbitrary::<u128>()
                .for_each(|&value| {
                    let mut buf = Vec::new();
                    encode(value, &mut buf);
                    let (arr, len) = encode_array(value);
                    assert_eq!(arr.get(..len), Some(buf.as_slice()));
                });
        }

        #[test]
        #[cfg_attr(miri, ignore)]
        fn decode_never_panics() {
            bolero::check!()
                .with_arbitrary::<Vec<u8>>()
                .for_each(|buf| {
                    // Should return Ok or Err, never panic
                    let _ = decode(buf);
                });
        }

        /// `a < b ⟹ encode(a) < encode(b)` lexicographically.
        #[test]
        #[cfg_attr(miri, ignore)]
        fn lexicographic_order() {
            bolero::check!()
                .with_arbitrary::<(u128, u128)>()
                .for_each(|&(a, b)| {
                    let (enc_a, len_a) = encode_array(a);
                    let (enc_b, len_b) = encode_array(b);
                    let slice_a = &enc_a[..len_a];
                    let slice_b = &enc_b[..len_b];
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
    }
}
