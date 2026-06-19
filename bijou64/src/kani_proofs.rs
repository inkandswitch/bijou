//! Kani proof harnesses.
//!
//! These verify `decode` — the one core function that the Aeneas
//! pipeline cannot translate (its `&[a, ..]` subslice rest-patterns are
//! unsupported; see `../../.ignore/aeneas/SPIKE.md`). Kani handles all
//! of Rust, including those patterns, via bounded model checking.
//!
//! Decode reads at most `MAX_BYTES` (9) bytes and ignores any trailing
//! input, so a symbolic buffer of length `0..=MAX_BYTES` exercises every
//! control-flow path. Harnesses are `#[cfg(kani)]`, so they never enter
//! a normal build, test, or clippy run.
//!
//! Run with upstream Kani (not packaged for NixOS — run elsewhere or in
//! the Kani container):
//!
//! ```sh
//! cargo kani -p bijou64
//! ```
#![allow(clippy::indexing_slicing, clippy::unwrap_used)]

use crate::{DecodeError, MAX_BYTES, TAG_THRESHOLD, decode, encode, encoded_bytes, encoded_len};
use alloc::vec::Vec;

/// A symbolic buffer of length `0..=MAX_BYTES`: enough to drive every
/// path through `decode`, since it never inspects beyond byte `MAX_BYTES`.
fn symbolic_buf() -> ([u8; MAX_BYTES], usize) {
    let bytes: [u8; MAX_BYTES] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_BYTES);
    (bytes, len)
}

/// `decode` is total: it never panics, overflows, or indexes
/// out of bounds on any input. (Kani checks these automatically.)
#[kani::proof]
fn decode_never_panics() {
    let (bytes, len) = symbolic_buf();
    let _ = decode(&bytes[..len]);
}

/// Whatever `decode` accepts, it consumes a sane number of bytes:
/// at least the tag, never more than the buffer, never more than
/// `MAX_BYTES`.
#[kani::proof]
fn decode_consumes_within_bounds() {
    let (bytes, len) = symbolic_buf();
    if let Ok((_, consumed)) = decode(&bytes[..len]) {
        assert!(consumed >= 1);
        assert!(consumed <= len);
        assert!(consumed <= MAX_BYTES);
    }
}

/// Canonicality, by construction: anything `decode` accepts is exactly
/// the encoding of the value it returns. There are no overlong
/// encodings to accept. This is the property whose Rust shape blocks
/// Aeneas, so verifying it here is the point of the Kani layer.
#[kani::proof]
fn decode_accepts_only_canonical() {
    let (bytes, len) = symbolic_buf();
    if let Ok((value, consumed)) = decode(&bytes[..len]) {
        let re = encoded_bytes(value);
        assert!(re.len() == consumed);
        assert!(re.as_slice() == &bytes[..consumed]);
    }
}

/// Round-trip: every value encodes to bytes that decode back to it,
/// consuming exactly the encoded length.
#[kani::proof]
fn roundtrip_encode_decode() {
    let value: u64 = kani::any();
    let enc = encoded_bytes(value);
    let (decoded, consumed) = decode(enc.as_slice()).unwrap();
    assert!(decoded == value);
    assert!(consumed == enc.len());
}

/// Round-trip through the allocating `encode`/`Vec` path (the API most
/// callers use) — distinct from `encoded_bytes`, so it needs its own
/// check. May require `#[kani::unwind(MAX_BYTES + 2)]` when run, since
/// `Vec` growth is a loop.
#[kani::proof]
fn roundtrip_vec_encode_decode() {
    let value: u64 = kani::any();
    let mut buf = Vec::new();
    encode(value, &mut buf);
    let (decoded, consumed) = decode(&buf).unwrap();
    assert!(decoded == value);
    assert!(consumed == buf.len());
}

/// `encoded_len` agrees with the actual encoded length and stays within
/// `1..=MAX_BYTES`. (Aeneas only *translates* `encoded_len`; this is its
/// only correctness check.)
#[kani::proof]
fn encoded_len_matches_encoding() {
    let value: u64 = kani::any();
    let len = encoded_len(value);
    assert!(len >= 1);
    assert!(len <= MAX_BYTES);
    assert!(len == encoded_bytes(value).len());
}

/// Length is determined entirely by the first byte: the bytes a
/// successful `decode` consumes are a function of the tag alone (the
/// SPEC's O(1)-skip property). Mirrors `Bijou.Family.decode_consumed_from_tag`.
#[kani::proof]
fn decode_length_from_first_byte() {
    let (bytes, len) = symbolic_buf();
    if let Ok((_, consumed)) = decode(&bytes[..len]) {
        let tag = bytes[0]; // len >= 1, since decode succeeded
        let expected = if tag < TAG_THRESHOLD {
            1
        } else {
            (tag as usize - TAG_THRESHOLD as usize) + 2
        };
        assert!(consumed == expected);
    }
}

/// Trailing bytes are ignored: appending arbitrary data after a valid
/// encoding changes neither the decoded value nor the consumed length.
#[kani::proof]
fn roundtrip_ignores_trailing() {
    let value: u64 = kani::any();
    let enc = encoded_bytes(value);
    let n = enc.len();

    let mut buf = [0u8; MAX_BYTES * 2];
    let tail: [u8; MAX_BYTES] = kani::any();
    buf[..n].copy_from_slice(enc.as_slice());
    buf[n..n + MAX_BYTES].copy_from_slice(&tail);

    let (decoded, consumed) = decode(&buf[..n + MAX_BYTES]).unwrap();
    assert!(decoded == value);
    assert!(consumed == n);
}

/// The two error conditions are the only ones, and they are
/// distinguishable exactly as the SPEC says: a short buffer is rejected,
/// and only tier 8 (tag `0xFF`) can overflow.
#[kani::proof]
fn decode_errors_match_spec() {
    let (bytes, len) = symbolic_buf();
    match decode(&bytes[..len]) {
        Err(DecodeError::BufferTooShort) => {
            // Either empty, or the tag demands more payload bytes than provided.
            // A multi-byte tag needs `tag - (TAG_THRESHOLD - 1)` payload bytes.
            assert!(
                len == 0
                    || (bytes[0] >= TAG_THRESHOLD
                        && len < (bytes[0] as usize - (TAG_THRESHOLD as usize - 1)) + 1)
            );
        }
        Err(DecodeError::Overflow) => {
            // Overflow is reachable only at the top tier (tag 0xFF).
            assert!(bytes[0] == u8::MAX);
        }
        Ok(_) => {}
    }
}
