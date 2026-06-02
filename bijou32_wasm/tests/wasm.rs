//! Wasm-target integration tests for `bijou32_wasm`.
//!
//! Compiled to `wasm32-unknown-unknown` by `wasm-pack test --node` and run
//! in Node.js. This layer exercises the Rust ↔ wasm-bindgen ABI directly
//! (without going through the wasm-bodge dist build). Cross-environment
//! coverage of the actual `dist/` package lives at the JS layer:
//!
//! - `test:js:32:node`    — Mocha against `dist/esm/node.js`
//! - `test:js:32:browser` — Playwright against `dist/esm/web.js` across
//!   chromium / firefox / webkit
//!
//! Run locally:
//!
//! ```sh
//! wasm-pack test --node bijou32_wasm
//! ```

#![cfg(target_family = "wasm")]
#![allow(clippy::missing_panics_doc, clippy::unwrap_used)]

use bijou32_wasm::{
    decode::{decode, decode_all},
    encode::{encode, encoded_len},
    max_bytes,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

/// Build a JS number from a Rust `u32`.
fn n(v: u32) -> JsValue {
    JsValue::from_f64(f64::from(v))
}

/// Build a JS number from a Rust `f64` — used for fractional, NaN,
/// out-of-range, and negative-number cases.
fn nf(v: f64) -> JsValue {
    JsValue::from_f64(v)
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
fn max_bytes_is_five() {
    assert_eq!(max_bytes(), 5);
}

#[wasm_bindgen_test]
fn tier_0_single_byte_encoding() {
    // For values < 252 the byte _is_ the value.
    assert_eq!(encode(&n(0)).unwrap(), vec![0x00]);
    assert_eq!(encode(&n(42)).unwrap(), vec![0x2A]);
    assert_eq!(encode(&n(251)).unwrap(), vec![0xFB]);
}

#[wasm_bindgen_test]
fn tier_1_uses_offset() {
    // Tier 1: tag 0xFC, payload = value - 252.
    assert_eq!(encode(&n(252)).unwrap(), vec![0xFC, 0x00]);
    assert_eq!(encode(&n(300)).unwrap(), vec![0xFC, 0x30]);
    assert_eq!(encode(&n(507)).unwrap(), vec![0xFC, 0xFF]);
}

#[wasm_bindgen_test]
fn u32_max_uses_full_five_bytes() {
    let bytes = encode(&n(u32::MAX)).unwrap();
    assert_eq!(bytes.len(), 5);
    assert_eq!(bytes[0], 0xFF);
}

#[wasm_bindgen_test]
fn encoded_len_matches_encode_len() {
    let cases: &[u32] = &[
        0,
        251,
        252,
        507,
        508,
        65_535,
        66_043,
        66_044,
        16_843_259,
        16_843_260,
        1u32 << 24,
        u32::MAX - 1,
        u32::MAX,
    ];

    for &v in cases {
        let jv = n(v);
        let computed = encoded_len(&jv).unwrap();
        let actual = encode(&jv).unwrap().len();
        assert_eq!(
            computed, actual,
            "encoded_len({v}) = {computed} but encode produced {actual} bytes",
        );
    }
}

#[wasm_bindgen_test]
fn decode_round_trip() {
    let cases: &[u32] = &[
        0,
        1,
        251,
        252,
        507,
        508,
        65_535,
        66_043,
        66_044,
        16_843_259,
        16_843_260,
        1u32 << 24,
        u32::MAX - 1,
        u32::MAX,
    ];

    for &v in cases {
        let bytes = encode(&n(v)).unwrap();
        let result = decode(&u8s(&bytes)).unwrap();
        assert_eq!(result.value(), v, "round-trip failed for {v}");
        assert_eq!(result.bytes_read(), bytes.len());
    }
}

#[wasm_bindgen_test]
fn decode_partial_buffer_reports_bytes_read() {
    let mut buf = encode(&n(300)).unwrap(); // 2 bytes
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
    // Tag 0xFC needs 1 payload byte — supply only the tag.
    assert!(decode(&u8s(&[0xFC])).is_err());
}

#[wasm_bindgen_test]
fn decode_truncated_tier_4_errors() {
    // Tag 0xFF needs 4 payload bytes — supply 3.
    assert!(decode(&u8s(&[0xFF, 0, 0, 0])).is_err());
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
    for bad in [JsValue::NULL, JsValue::from(42u32), JsValue::from_str("nope")] {
        let err = decode(&bad).expect_err("non-Uint8Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }
}

// ---- Range-check tests for the number → u32 boundary ----------------------

#[wasm_bindgen_test]
fn encode_rejects_value_equal_to_two_to_the_thirty_second() {
    // 2^32 — exactly one past u32::MAX. Without the validation, this
    // would silently truncate to 0 and encode as [0x00].
    let err = encode(&nf(4_294_967_296.0)).expect_err("must reject 2^32");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_rejects_negative_one() {
    // -1. Without validation, JS's `>>> 0` cast encodes this as
    // u32::MAX — a real footgun for content-addressed protocols.
    let err = encode(&nf(-1.0)).expect_err("must reject -1");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_rejects_large_negative() {
    let err = encode(&nf(-2_147_483_648.0)).expect_err("must reject -2^31");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_rejects_fractional() {
    // Fractional values should be rejected as TypeError, not
    // RangeError — they are nominally in range but not integers.
    let err = encode(&nf(1.5)).expect_err("must reject 1.5");
    assert_eq!(js_error_name(&err).as_deref(), Some("TypeError"));
}

#[wasm_bindgen_test]
fn encode_rejects_nan_and_infinity() {
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let err = encode(&nf(v)).expect_err("must reject NaN/Infinity");
        assert_eq!(
            js_error_name(&err).as_deref(),
            Some("TypeError"),
            "v = {v:?}"
        );
    }
}

#[wasm_bindgen_test]
fn encoded_len_rejects_out_of_range() {
    let err = encoded_len(&nf(4_294_967_296.0)).expect_err("must reject 2^32");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));

    let err = encoded_len(&nf(-1.0)).expect_err("must reject -1");
    assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
}

#[wasm_bindgen_test]
fn encode_accepts_u32_max_exactly() {
    // The largest accepted value: u32::MAX = 2^32 - 1.
    let bytes = encode(&n(u32::MAX)).expect("u32::MAX must be accepted");
    assert_eq!(bytes.len(), 5);
    assert_eq!(bytes[0], 0xFF);
}

#[wasm_bindgen_test]
fn encode_accepts_zero_exactly() {
    let bytes = encode(&n(0)).expect("0 must be accepted");
    assert_eq!(bytes, vec![0x00]);
}

// ---- decode_all batch helper -----------------------------------------------

#[wasm_bindgen_test]
fn decode_all_empty_returns_empty() {
    let result = decode_all(&u8s(&[])).expect("empty input decodes to empty array");
    assert!(result.is_empty());
}

#[wasm_bindgen_test]
fn decode_all_multi_value_roundtrip() {
    let mut buf = Vec::new();
    for v in [42u32, 300, 65_535] {
        buf.extend_from_slice(&encode(&n(v)).unwrap());
    }
    let result = decode_all(&u8s(&buf)).expect("must decode all");
    assert_eq!(result, vec![42u32, 300, 65_535]);
}

#[wasm_bindgen_test]
fn decode_all_propagates_error() {
    // [0x42, 0xFC] — the first byte decodes, the second is a tag without
    // payload. decode_all must surface the error and not return the partial
    // prefix.
    assert!(decode_all(&u8s(&[0x42, 0xFC])).is_err());
}

#[wasm_bindgen_test]
fn decode_all_propagates_overflow() {
    // Tier-4 all-ones overflows.
    assert!(decode_all(&u8s(&[0xFFu8; 5])).is_err());
}
