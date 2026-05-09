//! Wasm-target integration tests for `bijou64_wasm`.
//!
//! Compiled to `wasm32-unknown-unknown` by `wasm-pack test` and run inside
//! the host runtime (Node.js or a real browser via `--headless --chrome`,
//! `--firefox`, etc.). This complements the host-side Rust tests in
//! `bijou64` and the JS-side Playwright tests in `e2e/` — together they
//! cover Rust ↔ wasm-bindgen ↔ JS at every layer.
//!
//! Run locally:
//!
//! ```sh
//! wasm:test:node       # Node.js (fast, no browser dependencies)
//! wasm:test:chrome     # Headless Chromium
//! ```

#![cfg(target_family = "wasm")]
#![allow(clippy::missing_panics_doc, clippy::unwrap_used)]

use bijou64_wasm::{decode, encode, encoded_len, max_bytes};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn max_bytes_is_nine() {
    assert_eq!(max_bytes(), 9);
}

#[wasm_bindgen_test]
fn tier_0_single_byte_encoding() {
    // For values < 248 the byte _is_ the value.
    assert_eq!(encode(0), vec![0x00]);
    assert_eq!(encode(42), vec![0x2A]);
    assert_eq!(encode(247), vec![0xF7]);
}

#[wasm_bindgen_test]
fn tier_1_uses_offset() {
    // Tier 1: tag 0xF8, payload = value - 248.
    assert_eq!(encode(248), vec![0xF8, 0x00]);
    assert_eq!(encode(300), vec![0xF8, 0x34]);
    assert_eq!(encode(503), vec![0xF8, 0xFF]);
}

#[wasm_bindgen_test]
fn u64_max_uses_full_nine_bytes() {
    let bytes = encode(u64::MAX);
    assert_eq!(bytes.len(), 9);
    assert_eq!(bytes[0], 0xFF);
}

#[wasm_bindgen_test]
fn encoded_len_matches_encode_len() {
    // The two should agree across every tier boundary plus a few interior
    // points. If they diverge for any input, the format is internally
    // inconsistent.
    let cases = [
        0,
        247,
        248,
        503,
        504,
        65_535,
        66_039,
        66_040,
        16_843_255,
        1_u64 << 32,
        u64::MAX - 1,
        u64::MAX,
    ];

    for v in cases {
        let computed = encoded_len(v);
        let actual = encode(v).len();
        assert_eq!(
            computed, actual,
            "encoded_len({v}) = {computed} but encode produced {actual} bytes",
        );
    }
}

#[wasm_bindgen_test]
fn decode_round_trip() {
    // Spot-check each tier boundary plus u64::MAX. Exhaustive coverage
    // lives in the host-side property tests; here we just verify the JS
    // glue doesn't lose bits at the boundary.
    let cases: &[u64] = &[
        0,
        1,
        247,
        248,
        503,
        504,
        65_535,
        66_039,
        66_040,
        16_843_255,
        1_u64 << 32,
        u64::MAX - 1,
        u64::MAX,
    ];

    for &v in cases {
        let bytes = encode(v);
        let result = decode(&bytes).unwrap();
        assert_eq!(result.value(), v, "round-trip failed for {v}");
        assert_eq!(result.bytes_read(), bytes.len());
    }
}

#[wasm_bindgen_test]
fn decode_partial_buffer_reports_bytes_read() {
    // bytesRead should be the encoding length, not the input length —
    // this is what allows stream-decoding by repeatedly slicing.
    let mut buf = encode(300); // 2 bytes
    buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    let result = decode(&buf).unwrap();
    assert_eq!(result.value(), 300);
    assert_eq!(result.bytes_read(), 2);
}

#[wasm_bindgen_test]
fn decode_empty_input_errors() {
    assert!(decode(&[]).is_err());
}

#[wasm_bindgen_test]
fn decode_truncated_tier_1_errors() {
    // Tag 0xF8 needs 1 payload byte — supply only the tag.
    assert!(decode(&[0xF8]).is_err());
}

#[wasm_bindgen_test]
fn decode_truncated_tier_8_errors() {
    // Tag 0xFF needs 8 payload bytes — supply 7.
    assert!(decode(&[0xFF, 0, 0, 0, 0, 0, 0, 0]).is_err());
}
