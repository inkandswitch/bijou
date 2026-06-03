//! Wasm-target integration tests for `bijou128_wasm`.
//!
//! Compiled to `wasm32-unknown-unknown` by `wasm-pack test --node` and run
//! in Node.js. This layer exercises the Rust ↔ wasm-bindgen ABI directly
//! (without going through the wasm-bodge dist build). Cross-environment
//! coverage of the actual `dist/` package lives at the JS layer:
//!
//! - `test:js:128:node`    — Mocha against `dist/esm/node.js` (CJS path is
//!   covered by `dist/cjs/node.cjs`)
//! - `test:js:128:browser` — Playwright against `dist/esm/web.js` across
//!   chromium / firefox / webkit
//!
//! Run locally:
//!
//! ```sh
//! wasm-pack test --node bijou128_wasm   # this file, Rust ABI in Node
//! test:js:128                            # node + browsers, JS surface against `dist/`
//! ```

#![cfg(target_family = "wasm")]
#![allow(clippy::missing_panics_doc, clippy::unwrap_used)]

use bijou128_wasm::{
    decode::{decode, decode_all},
    encode::{encode, encoded_len},
    max_bytes,
};
use js_sys::BigInt;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

/// Build a `BigInt` from a Rust `u128`. wasm-bindgen has an `Into<JsValue>`
/// impl for `u128` (two-word marshalling) that we route through to get a
/// real JS `bigint`.
fn bi(v: u128) -> BigInt {
    BigInt::unchecked_from_js(JsValue::from(v))
}

/// Build a `BigInt` from a decimal string. Used for values outside the
/// `u128` range — there is no `From<i128>` shortcut for negatives and we
/// want to test exactly `2**128` and similar boundaries.
fn bi_str(s: &str) -> BigInt {
    BigInt::new(&JsValue::from_str(s)).expect("valid bigint literal")
}

/// Pull `name` off any throwable so we can assert on it.
fn js_error_name<E: Clone + Into<JsValue>>(err: &E) -> Option<String> {
    let v: JsValue = err.clone().into();
    let e: &js_sys::Error = v.dyn_ref()?;
    e.name().as_string()
}

/// Pull `name` off a raw `JsValue` throwable.
fn js_value_error_name(v: &JsValue) -> Option<String> {
    v.dyn_ref::<js_sys::Error>()?.name().as_string()
}

/// Build a JS `Uint8Array` from a byte slice as a `JsValue`. The decode
/// entry points take `&JsValue` (and validate it is a `Uint8Array`), so
/// tests marshal through one rather than passing a Rust slice directly.
fn u8s(bytes: &[u8]) -> JsValue {
    js_sys::Uint8Array::from(bytes).into()
}

#[wasm_bindgen_test]
fn max_bytes_is_seventeen() {
    assert_eq!(max_bytes(), 17);
}

#[wasm_bindgen_test]
fn tier_0_single_byte_encoding() {
    // For values < 240 the byte _is_ the value.
    assert_eq!(encode(&bi(0)).unwrap(), vec![0x00]);
    assert_eq!(encode(&bi(42)).unwrap(), vec![0x2A]);
    assert_eq!(encode(&bi(239)).unwrap(), vec![0xEF]);
}

#[wasm_bindgen_test]
fn tier_1_uses_offset() {
    // Tier 1: tag 0xF0, payload = value - 240.
    assert_eq!(encode(&bi(240)).unwrap(), vec![0xF0, 0x00]);
    assert_eq!(encode(&bi(300)).unwrap(), vec![0xF0, 0x3C]);
    assert_eq!(encode(&bi(495)).unwrap(), vec![0xF0, 0xFF]);
}

#[wasm_bindgen_test]
fn u128_max_uses_full_seventeen_bytes() {
    let bytes = encode(&bi(u128::MAX)).unwrap();
    assert_eq!(bytes.len(), 17);
    assert_eq!(bytes[0], 0xFF);
}

#[wasm_bindgen_test]
fn encoded_len_matches_encode_len() {
    // Spot-check tier boundaries plus mid-range probes.
    let cases: &[u128] = &[
        0,
        239,
        240,
        495,
        496,
        65_535,
        66_031,
        66_032,
        16_843_247,
        1u128 << 32,
        1u128 << 64,
        1u128 << 96,
        u128::MAX - 1,
        u128::MAX,
    ];

    for &v in cases {
        let bv = bi(v);
        let computed = encoded_len(&bv).unwrap();
        let actual = encode(&bv).unwrap().len();
        assert_eq!(
            computed, actual,
            "encoded_len({v}) = {computed} but encode produced {actual} bytes",
        );
    }
}

#[wasm_bindgen_test]
fn decode_round_trip() {
    let cases: &[u128] = &[
        0,
        1,
        239,
        240,
        495,
        496,
        65_535,
        66_031,
        66_032,
        16_843_247,
        1u128 << 32,
        1u128 << 64,
        1u128 << 96,
        u128::MAX - 1,
        u128::MAX,
    ];

    for &v in cases {
        let bytes = encode(&bi(v)).unwrap();
        let result = decode(&u8s(&bytes)).unwrap();
        assert_eq!(result.value(), v, "round-trip failed for {v}");
        assert_eq!(result.bytes_read(), bytes.len());
    }
}

#[wasm_bindgen_test]
fn decode_partial_buffer_reports_bytes_read() {
    // bytesRead should be the encoding length, not the input length —
    // this is what allows stream-decoding by repeatedly slicing.
    let mut buf = encode(&bi(300)).unwrap(); // 2 bytes
    buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    let result = decode(&u8s(&buf)).unwrap();
    assert_eq!(result.value(), 300);
    assert_eq!(result.bytes_read(), 2);
}

#[wasm_bindgen_test]
fn decode_empty_input_errors() {
    assert!(decode(&u8s(&[])).is_err());
}

#[wasm_bindgen_test]
fn decode_truncated_tier_1_errors() {
    // Tag 0xF0 needs 1 payload byte — supply only the tag.
    assert!(decode(&u8s(&[0xF0])).is_err());
}

#[wasm_bindgen_test]
fn decode_truncated_tier_16_errors() {
    // Tag 0xFF needs 16 payload bytes — supply 15.
    assert!(decode(&u8s(&[0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])).is_err());
}

// ---- input-type guard tests (non-Uint8Array rejection) --------------------

#[wasm_bindgen_test]
fn decode_rejects_plain_array() {
    // A plain JS `Array` is not a `Uint8Array`. The default
    // `&[u8]`/`&Uint8Array` marshalling would coerce it via
    // `new Uint8Array(arr)`, silently truncating out-of-range elements.
    // We now reject it with a TypeError.
    let plain = js_sys::Array::of1(&JsValue::from(0u8));
    let err = decode(plain.as_ref()).expect_err("plain Array must be rejected");
    assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
}

#[wasm_bindgen_test]
fn decode_rejects_out_of_range_array_element_without_truncation() {
    // [1000] as a plain Array. The old behaviour silently decoded this
    // as value 232 (1000 & 0xFF). It must now throw, never truncate.
    let plain = js_sys::Array::of1(&JsValue::from(1000u32));
    let err = decode(plain.as_ref()).expect_err("out-of-range Array element must be rejected");
    assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
}

#[wasm_bindgen_test]
fn decode_all_rejects_plain_array() {
    let plain = js_sys::Array::of2(&JsValue::from(0u8), &JsValue::from(1u8));
    let err = decode_all(plain.as_ref()).expect_err("plain Array must be rejected");
    assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
}

#[wasm_bindgen_test]
fn decode_rejects_non_array_inputs() {
    for bad in [
        JsValue::NULL,
        JsValue::from(42u32),
        JsValue::from_str("nope"),
    ] {
        let err = decode(&bad).expect_err("non-Uint8Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }
}

// ---- Range-check tests for the bigint → u128 boundary ---------------------

#[wasm_bindgen_test]
fn encode_rejects_value_equal_to_two_to_the_one_twenty_eighth() {
    // 2^128 — exactly one past u128::MAX. Without the validation, this
    // would silently truncate to 0 and encode as [0x00].
    let too_big = bi_str("340282366920938463463374607431768211456");
    let err = encode(&too_big).expect_err("must reject 2^128");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_rejects_negative_one() {
    // -1n. Without validation, two's-complement wraparound encodes this
    // as u128::MAX — a real footgun for content-addressed protocols.
    let neg_one = bi_str("-1");
    let err = encode(&neg_one).expect_err("must reject -1n");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_rejects_large_negative() {
    let neg = bi_str("-170141183460469231731687303715884105728"); // -2^127
    let err = encode(&neg).expect_err("must reject -2^127");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encoded_len_rejects_out_of_range() {
    let too_big = bi_str("340282366920938463463374607431768211456");
    let err = encoded_len(&too_big).expect_err("must reject 2^128");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));

    let neg = bi_str("-1");
    let err = encoded_len(&neg).expect_err("must reject -1n");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_accepts_u128_max_exactly() {
    // The largest accepted value: u128::MAX = 2^128 - 1.
    let max = bi_str("340282366920938463463374607431768211455");
    let bytes = encode(&max).expect("u128::MAX must be accepted");
    assert_eq!(bytes.len(), 17);
    assert_eq!(bytes[0], 0xFF);
}

#[wasm_bindgen_test]
fn encode_accepts_zero_exactly() {
    let zero = bi_str("0");
    let bytes = encode(&zero).expect("0 must be accepted");
    assert_eq!(bytes, vec![0x00]);
}

// ---- decode_all batch helper ----------------------------------------------

#[wasm_bindgen_test]
fn decode_all_empty_returns_empty() {
    let result = decode_all(&u8s(&[])).expect("empty input decodes to empty array");
    assert_eq!(result.length(), 0);
}

#[wasm_bindgen_test]
fn decode_all_multi_value_roundtrip() {
    let mut buf = Vec::new();
    for v in [42u128, 300, 65_535] {
        buf.extend_from_slice(&encode(&bi(v)).unwrap());
    }
    let result = decode_all(&u8s(&buf)).expect("must decode all");
    assert_eq!(result.length(), 3);

    // Each element should be a JS bigint matching the original value.
    for (i, expected) in [42u128, 300, 65_535].iter().enumerate() {
        let got: u128 =
            u128::try_from(result.get(i as u32)).expect("each element must be a u128 bigint");
        assert_eq!(got, *expected);
    }
}

#[wasm_bindgen_test]
fn decode_all_propagates_error() {
    // [0x42, 0xF0] — the first byte decodes, the second is a tag without
    // payload. decode_all must surface the error and not return the partial
    // prefix.
    assert!(decode_all(&u8s(&[0x42, 0xF0])).is_err());
}

#[wasm_bindgen_test]
fn decode_all_propagates_overflow() {
    // Tier-16 all-ones overflows.
    assert!(decode_all(&u8s(&[0xFFu8; 17])).is_err());
}
