//! Wasm-target integration tests for `bijoux_wasm` — all three widths.
//!
//! Compiled to `wasm32-unknown-unknown` by `wasm-pack test --node` and
//! run in Node.js, exercising the Rust ↔ wasm-bindgen ABI directly.
//! JS-package coverage of the actual `dist/` build lives in `test/`
//! (Mocha) and `e2e/` (Playwright).

#![cfg(target_family = "wasm")]
#![allow(
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    clippy::unwrap_used
)]

mod u32 {

    use bijoux_wasm::bijou32::{
        decode::{decode_all_u32, decode_u32},
        encode::{encode_u32, encoded_len_u32},
        max_bytes_u32,
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
        assert_eq!(max_bytes_u32(), 5);
    }

    #[wasm_bindgen_test]
    fn tier_0_single_byte_encoding() {
        // For values < 252 the byte _is_ the value.
        assert_eq!(encode_u32(&n(0)).unwrap(), vec![0x00]);
        assert_eq!(encode_u32(&n(42)).unwrap(), vec![0x2A]);
        assert_eq!(encode_u32(&n(251)).unwrap(), vec![0xFB]);
    }

    #[wasm_bindgen_test]
    fn tier_1_uses_offset() {
        // Tier 1: tag 0xFC, payload = value - 252.
        assert_eq!(encode_u32(&n(252)).unwrap(), vec![0xFC, 0x00]);
        assert_eq!(encode_u32(&n(300)).unwrap(), vec![0xFC, 0x30]);
        assert_eq!(encode_u32(&n(507)).unwrap(), vec![0xFC, 0xFF]);
    }

    #[wasm_bindgen_test]
    fn u32_max_uses_full_five_bytes() {
        let bytes = encode_u32(&n(u32::MAX)).unwrap();
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
            let computed = encoded_len_u32(&jv).unwrap();
            let actual = encode_u32(&jv).unwrap().len();
            assert_eq!(
                computed, actual,
                "encoded_len_u32({v}) = {computed} but encode produced {actual} bytes",
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
            let bytes = encode_u32(&n(v)).unwrap();
            let result = decode_u32(&u8s(&bytes)).unwrap();
            assert_eq!(result.value(), v, "round-trip failed for {v}");
            assert_eq!(result.bytes_read(), bytes.len());
        }
    }

    #[wasm_bindgen_test]
    fn decode_partial_buffer_reports_bytes_read() {
        let mut buf = encode_u32(&n(300)).unwrap(); // 2 bytes
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let result = decode_u32(&u8s(&buf)).unwrap();
        assert_eq!(result.value(), 300);
        assert_eq!(result.bytes_read(), 2);
    }

    #[wasm_bindgen_test]
    fn decode_empty_input_errors() {
        assert!(decode_u32(&u8s(&[])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_truncated_tier_1_errors() {
        // Tag 0xFC needs 1 payload byte — supply only the tag.
        assert!(decode_u32(&u8s(&[0xFC])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_truncated_tier_4_errors() {
        // Tag 0xFF needs 4 payload bytes — supply 3.
        assert!(decode_u32(&u8s(&[0xFF, 0, 0, 0])).is_err());
    }

    // ---- input-type guard tests (non-Uint8Array rejection) --------------------

    #[wasm_bindgen_test]
    fn decode_rejects_plain_array() {
        // A plain JS `Array` is not a `Uint8Array`. The default
        // `&[u8]`/`&Uint8Array` marshalling would coerce it via
        // `new Uint8Array(arr)`, silently truncating out-of-range elements.
        // We now reject it with a TypeError.
        let plain = js_sys::Array::of1(&JsValue::from(0u8));
        let err = decode_u32(plain.as_ref()).expect_err("plain Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_rejects_out_of_range_array_element_without_truncation() {
        // [1000] as a plain Array. The old behaviour silently decoded this
        // as value 232 (1000 & 0xFF). It must now throw, never truncate.
        let plain = js_sys::Array::of1(&JsValue::from(1000u32));
        let err =
            decode_u32(plain.as_ref()).expect_err("out-of-range Array element must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_all_rejects_plain_array() {
        let plain = js_sys::Array::of2(&JsValue::from(0u8), &JsValue::from(1u8));
        let err = decode_all_u32(plain.as_ref()).expect_err("plain Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_rejects_non_array_inputs() {
        for bad in [
            JsValue::NULL,
            JsValue::from(42u32),
            JsValue::from_str("nope"),
        ] {
            let err = decode_u32(&bad).expect_err("non-Uint8Array must be rejected");
            assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
        }
    }

    // ---- Range-check tests for the number → u32 boundary ----------------------

    #[wasm_bindgen_test]
    fn encode_rejects_value_equal_to_two_to_the_thirty_second() {
        // 2^32 — exactly one past u32::MAX. Without the validation, this
        // would silently truncate to 0 and encode as [0x00].
        let err = encode_u32(&nf(4_294_967_296.0)).expect_err("must reject 2^32");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_negative_one() {
        // -1. Without validation, JS's `>>> 0` cast encodes this as
        // u32::MAX — a real footgun for content-addressed protocols.
        let err = encode_u32(&nf(-1.0)).expect_err("must reject -1");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_large_negative() {
        let err = encode_u32(&nf(-2_147_483_648.0)).expect_err("must reject -2^31");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_fractional() {
        // Fractional values should be rejected as TypeError, not
        // RangeError — they are nominally in range but not integers.
        let err = encode_u32(&nf(1.5)).expect_err("must reject 1.5");
        assert_eq!(js_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_nan_and_infinity() {
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = encode_u32(&nf(v)).expect_err("must reject NaN/Infinity");
            assert_eq!(
                js_error_name(&err).as_deref(),
                Some("TypeError"),
                "v = {v:?}"
            );
        }
    }

    #[wasm_bindgen_test]
    fn encoded_len_rejects_out_of_range() {
        let err = encoded_len_u32(&nf(4_294_967_296.0)).expect_err("must reject 2^32");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));

        let err = encoded_len_u32(&nf(-1.0)).expect_err("must reject -1");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_accepts_u32_max_exactly() {
        // The largest accepted value: u32::MAX = 2^32 - 1.
        let bytes = encode_u32(&n(u32::MAX)).expect("u32::MAX must be accepted");
        assert_eq!(bytes.len(), 5);
        assert_eq!(bytes[0], 0xFF);
    }

    #[wasm_bindgen_test]
    fn encode_accepts_zero_exactly() {
        let bytes = encode_u32(&n(0)).expect("0 must be accepted");
        assert_eq!(bytes, vec![0x00]);
    }

    // ---- decode_all batch helper -----------------------------------------------

    #[wasm_bindgen_test]
    fn decode_all_empty_returns_empty() {
        let result = decode_all_u32(&u8s(&[])).expect("empty input decodes to empty array");
        assert!(result.is_empty());
    }

    #[wasm_bindgen_test]
    fn decode_all_multi_value_roundtrip() {
        let mut buf = Vec::new();
        for v in [42u32, 300, 65_535] {
            buf.extend_from_slice(&encode_u32(&n(v)).unwrap());
        }
        let result = decode_all_u32(&u8s(&buf)).expect("must decode all");
        assert_eq!(result, vec![42u32, 300, 65_535]);
    }

    #[wasm_bindgen_test]
    fn decode_all_propagates_error() {
        // [0x42, 0xFC] — the first byte decodes, the second is a tag without
        // payload. decode_all must surface the error and not return the partial
        // prefix.
        assert!(decode_all_u32(&u8s(&[0x42, 0xFC])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_all_propagates_overflow() {
        // Tier-4 all-ones overflows.
        assert!(decode_all_u32(&u8s(&[0xFFu8; 5])).is_err());
    }
}

mod u64 {

    use bijoux_wasm::bijou64::{
        decode::{decode_all_u64, decode_u64},
        encode::{encode_u64, encoded_len_u64},
        max_bytes_u64,
    };
    use js_sys::BigInt;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Build a `BigInt` from a Rust `u64`. The wasm-bindgen `From<u64> for
    /// BigInt` impl is exactly the right thing here — no truncation, no
    /// loss of precision, no exposure to the wraparound semantics we're
    /// guarding against in the public API.
    fn bi(v: u64) -> BigInt {
        BigInt::from(v)
    }

    /// Build a `BigInt` from a decimal string. Used for values outside the
    /// `u64` range — there is no `From<i64>` shortcut for negatives and we
    /// want to test exactly `2**64` and similar boundaries.
    fn bi_str(s: &str) -> BigInt {
        BigInt::new(&JsValue::from_str(s)).expect("valid bigint literal")
    }

    /// Pull `name` off any throwable so we can assert on it.
    ///
    /// Accepts anything `Clone + Into<JsValue>` so callers can pass the
    /// raw `WasmBigintError` returned from `encode`/`encodedLen` directly,
    /// without having to convert it themselves.
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
    fn max_bytes_is_nine() {
        assert_eq!(max_bytes_u64(), 9);
    }

    #[wasm_bindgen_test]
    fn tier_0_single_byte_encoding() {
        // For values < 248 the byte _is_ the value.
        assert_eq!(encode_u64(&bi(0)).unwrap(), vec![0x00]);
        assert_eq!(encode_u64(&bi(42)).unwrap(), vec![0x2A]);
        assert_eq!(encode_u64(&bi(247)).unwrap(), vec![0xF7]);
    }

    #[wasm_bindgen_test]
    fn tier_1_uses_offset() {
        // Tier 1: tag 0xF8, payload = value - 248.
        assert_eq!(encode_u64(&bi(248)).unwrap(), vec![0xF8, 0x00]);
        assert_eq!(encode_u64(&bi(300)).unwrap(), vec![0xF8, 0x34]);
        assert_eq!(encode_u64(&bi(503)).unwrap(), vec![0xF8, 0xFF]);
    }

    #[wasm_bindgen_test]
    fn u64_max_uses_full_nine_bytes() {
        let bytes = encode_u64(&bi(u64::MAX)).unwrap();
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
            let bv = bi(v);
            let computed = encoded_len_u64(&bv).unwrap();
            let actual = encode_u64(&bv).unwrap().len();
            assert_eq!(
                computed, actual,
                "encoded_len_u64({v}) = {computed} but encode produced {actual} bytes",
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
            let bytes = encode_u64(&bi(v)).unwrap();
            let result = decode_u64(&u8s(&bytes)).unwrap();
            assert_eq!(result.value(), v, "round-trip failed for {v}");
            assert_eq!(result.bytes_read(), bytes.len());
        }
    }

    #[wasm_bindgen_test]
    fn decode_partial_buffer_reports_bytes_read() {
        // bytesRead should be the encoding length, not the input length —
        // this is what allows stream-decoding by repeatedly slicing.
        let mut buf = encode_u64(&bi(300)).unwrap(); // 2 bytes
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let result = decode_u64(&u8s(&buf)).unwrap();
        assert_eq!(result.value(), 300);
        assert_eq!(result.bytes_read(), 2);
    }

    #[wasm_bindgen_test]
    fn decode_empty_input_errors() {
        assert!(decode_u64(&u8s(&[])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_truncated_tier_1_errors() {
        // Tag 0xF8 needs 1 payload byte — supply only the tag.
        assert!(decode_u64(&u8s(&[0xF8])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_truncated_tier_8_errors() {
        // Tag 0xFF needs 8 payload bytes — supply 7.
        assert!(decode_u64(&u8s(&[0xFF, 0, 0, 0, 0, 0, 0, 0])).is_err());
    }

    // ---- input-type guard tests (non-Uint8Array rejection) --------------------

    #[wasm_bindgen_test]
    fn decode_rejects_plain_array() {
        // A plain JS `Array` is not a `Uint8Array`. The default
        // `&[u8]`/`&Uint8Array` marshalling would coerce it via
        // `new Uint8Array(arr)`, silently truncating out-of-range elements.
        // We now reject it with a TypeError.
        let plain = js_sys::Array::of1(&JsValue::from(0u8));
        let err = decode_u64(plain.as_ref()).expect_err("plain Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_rejects_out_of_range_array_element_without_truncation() {
        // [1000] as a plain Array. The dangerous case: the old behaviour
        // silently decoded this as value 232 (1000 & 0xFF). It must now
        // throw a TypeError, never silently truncate.
        let plain = js_sys::Array::of1(&JsValue::from(1000u32));
        let err =
            decode_u64(plain.as_ref()).expect_err("out-of-range Array element must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_all_rejects_plain_array() {
        let plain = js_sys::Array::of2(&JsValue::from(0u8), &JsValue::from(1u8));
        let err = decode_all_u64(plain.as_ref()).expect_err("plain Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_rejects_non_array_inputs() {
        // null, a number, and a string are all wrong-type.
        for bad in [
            JsValue::NULL,
            JsValue::from(42u32),
            JsValue::from_str("nope"),
        ] {
            let err = decode_u64(&bad).expect_err("non-Uint8Array must be rejected");
            assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
        }
    }

    // ---- Range-check tests for the bigint → u64 boundary ----------------------

    #[wasm_bindgen_test]
    fn encode_rejects_value_equal_to_two_to_the_sixty_fourth() {
        // 2^64 — exactly one past u64::MAX. Without the validation, this
        // would silently truncate to 0 and encode as [0x00].
        let too_big = bi_str("18446744073709551616");
        let err = encode_u64(&too_big).expect_err("must reject 2^64");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_negative_one() {
        // -1n. Without validation, two's-complement wraparound encodes this
        // as u64::MAX — a real footgun for content-addressed protocols.
        let neg_one = bi_str("-1");
        let err = encode_u64(&neg_one).expect_err("must reject -1n");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_large_negative() {
        let neg = bi_str("-9223372036854775808"); // -2^63
        let err = encode_u64(&neg).expect_err("must reject -2^63");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encoded_len_rejects_out_of_range() {
        let too_big = bi_str("18446744073709551616");
        let err = encoded_len_u64(&too_big).expect_err("must reject 2^64");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));

        let neg = bi_str("-1");
        let err = encoded_len_u64(&neg).expect_err("must reject -1n");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_accepts_u64_max_exactly() {
        // The largest accepted value: u64::MAX = 2^64 - 1.
        let max = bi_str("18446744073709551615");
        let bytes = encode_u64(&max).expect("u64::MAX must be accepted");
        assert_eq!(bytes.len(), 9);
        assert_eq!(bytes[0], 0xFF);
    }

    #[wasm_bindgen_test]
    fn encode_accepts_zero_exactly() {
        // The smallest accepted value: 0.
        let zero = bi_str("0");
        let bytes = encode_u64(&zero).expect("0 must be accepted");
        assert_eq!(bytes, vec![0x00]);
    }

    // ---- decode_all batch helper -----------------------------------------------

    #[wasm_bindgen_test]
    fn decode_all_empty_returns_empty() {
        let result = decode_all_u64(&u8s(&[])).expect("empty input decodes to empty array");
        assert!(result.is_empty());
    }

    #[wasm_bindgen_test]
    fn decode_all_multi_value_roundtrip() {
        // Encode three values back-to-back, decode_all should recover them all.
        let mut buf = Vec::new();
        for v in [42u64, 300, 65_535] {
            buf.extend_from_slice(&encode_u64(&bi(v)).unwrap());
        }
        let result = decode_all_u64(&u8s(&buf)).expect("must decode all");
        assert_eq!(result, vec![42u64, 300, 65_535]);
    }

    #[wasm_bindgen_test]
    fn decode_all_propagates_error() {
        // [0x42, 0xF8] — the first byte decodes, the second is a tag without
        // payload. decode_all must surface the error and not return the partial
        // prefix. The error's JS-side `name === "DecodeError"` is verified
        // in the Playwright e2e suite; here we just confirm the boundary
        // returns Err.
        assert!(decode_all_u64(&u8s(&[0x42, 0xF8])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_all_propagates_overflow() {
        // Tier-8 all-ones overflows.
        assert!(decode_all_u64(&u8s(&[0xFFu8; 9])).is_err());
    }
}

mod u128 {

    use bijoux_wasm::bijou128::{
        decode::{decode_all_u128, decode_u128},
        encode::{encode_u128, encoded_len_u128},
        max_bytes_u128,
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
        assert_eq!(max_bytes_u128(), 17);
    }

    #[wasm_bindgen_test]
    fn tier_0_single_byte_encoding() {
        // For values < 240 the byte _is_ the value.
        assert_eq!(encode_u128(&bi(0)).unwrap(), vec![0x00]);
        assert_eq!(encode_u128(&bi(42)).unwrap(), vec![0x2A]);
        assert_eq!(encode_u128(&bi(239)).unwrap(), vec![0xEF]);
    }

    #[wasm_bindgen_test]
    fn tier_1_uses_offset() {
        // Tier 1: tag 0xF0, payload = value - 240.
        assert_eq!(encode_u128(&bi(240)).unwrap(), vec![0xF0, 0x00]);
        assert_eq!(encode_u128(&bi(300)).unwrap(), vec![0xF0, 0x3C]);
        assert_eq!(encode_u128(&bi(495)).unwrap(), vec![0xF0, 0xFF]);
    }

    #[wasm_bindgen_test]
    fn u128_max_uses_full_seventeen_bytes() {
        let bytes = encode_u128(&bi(u128::MAX)).unwrap();
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
            let computed = encoded_len_u128(&bv).unwrap();
            let actual = encode_u128(&bv).unwrap().len();
            assert_eq!(
                computed, actual,
                "encoded_len_u128({v}) = {computed} but encode produced {actual} bytes",
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
            let bytes = encode_u128(&bi(v)).unwrap();
            let result = decode_u128(&u8s(&bytes)).unwrap();
            assert_eq!(result.value(), v, "round-trip failed for {v}");
            assert_eq!(result.bytes_read(), bytes.len());
        }
    }

    #[wasm_bindgen_test]
    fn decode_partial_buffer_reports_bytes_read() {
        // bytesRead should be the encoding length, not the input length —
        // this is what allows stream-decoding by repeatedly slicing.
        let mut buf = encode_u128(&bi(300)).unwrap(); // 2 bytes
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let result = decode_u128(&u8s(&buf)).unwrap();
        assert_eq!(result.value(), 300);
        assert_eq!(result.bytes_read(), 2);
    }

    #[wasm_bindgen_test]
    fn decode_empty_input_errors() {
        assert!(decode_u128(&u8s(&[])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_truncated_tier_1_errors() {
        // Tag 0xF0 needs 1 payload byte — supply only the tag.
        assert!(decode_u128(&u8s(&[0xF0])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_truncated_tier_16_errors() {
        // Tag 0xFF needs 16 payload bytes — supply 15.
        assert!(decode_u128(&u8s(&[0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])).is_err());
    }

    // ---- input-type guard tests (non-Uint8Array rejection) --------------------

    #[wasm_bindgen_test]
    fn decode_rejects_plain_array() {
        // A plain JS `Array` is not a `Uint8Array`. The default
        // `&[u8]`/`&Uint8Array` marshalling would coerce it via
        // `new Uint8Array(arr)`, silently truncating out-of-range elements.
        // We now reject it with a TypeError.
        let plain = js_sys::Array::of1(&JsValue::from(0u8));
        let err = decode_u128(plain.as_ref()).expect_err("plain Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_rejects_out_of_range_array_element_without_truncation() {
        // [1000] as a plain Array. The old behaviour silently decoded this
        // as value 232 (1000 & 0xFF). It must now throw, never truncate.
        let plain = js_sys::Array::of1(&JsValue::from(1000u32));
        let err =
            decode_u128(plain.as_ref()).expect_err("out-of-range Array element must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_all_rejects_plain_array() {
        let plain = js_sys::Array::of2(&JsValue::from(0u8), &JsValue::from(1u8));
        let err = decode_all_u128(plain.as_ref()).expect_err("plain Array must be rejected");
        assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
    }

    #[wasm_bindgen_test]
    fn decode_rejects_non_array_inputs() {
        for bad in [
            JsValue::NULL,
            JsValue::from(42u32),
            JsValue::from_str("nope"),
        ] {
            let err = decode_u128(&bad).expect_err("non-Uint8Array must be rejected");
            assert_eq!(js_value_error_name(&err).as_deref(), Some("TypeError"));
        }
    }

    // ---- Range-check tests for the bigint → u128 boundary ---------------------

    #[wasm_bindgen_test]
    fn encode_rejects_value_equal_to_two_to_the_one_twenty_eighth() {
        // 2^128 — exactly one past u128::MAX. Without the validation, this
        // would silently truncate to 0 and encode as [0x00].
        let too_big = bi_str("340282366920938463463374607431768211456");
        let err = encode_u128(&too_big).expect_err("must reject 2^128");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_negative_one() {
        // -1n. Without validation, two's-complement wraparound encodes this
        // as u128::MAX — a real footgun for content-addressed protocols.
        let neg_one = bi_str("-1");
        let err = encode_u128(&neg_one).expect_err("must reject -1n");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_rejects_large_negative() {
        let neg = bi_str("-170141183460469231731687303715884105728"); // -2^127
        let err = encode_u128(&neg).expect_err("must reject -2^127");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encoded_len_rejects_out_of_range() {
        let too_big = bi_str("340282366920938463463374607431768211456");
        let err = encoded_len_u128(&too_big).expect_err("must reject 2^128");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));

        let neg = bi_str("-1");
        let err = encoded_len_u128(&neg).expect_err("must reject -1n");
        assert_eq!(js_error_name(&err).as_deref(), Some("RangeError"));
    }

    #[wasm_bindgen_test]
    fn encode_accepts_u128_max_exactly() {
        // The largest accepted value: u128::MAX = 2^128 - 1.
        let max = bi_str("340282366920938463463374607431768211455");
        let bytes = encode_u128(&max).expect("u128::MAX must be accepted");
        assert_eq!(bytes.len(), 17);
        assert_eq!(bytes[0], 0xFF);
    }

    #[wasm_bindgen_test]
    fn encode_accepts_zero_exactly() {
        let zero = bi_str("0");
        let bytes = encode_u128(&zero).expect("0 must be accepted");
        assert_eq!(bytes, vec![0x00]);
    }

    // ---- decode_all batch helper ----------------------------------------------

    #[wasm_bindgen_test]
    fn decode_all_empty_returns_empty() {
        let result = decode_all_u128(&u8s(&[])).expect("empty input decodes to empty array");
        assert_eq!(result.length(), 0);
    }

    #[wasm_bindgen_test]
    fn decode_all_multi_value_roundtrip() {
        let mut buf = Vec::new();
        for v in [42u128, 300, 65_535] {
            buf.extend_from_slice(&encode_u128(&bi(v)).unwrap());
        }
        let result = decode_all_u128(&u8s(&buf)).expect("must decode all");
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
        assert!(decode_all_u128(&u8s(&[0x42, 0xF0])).is_err());
    }

    #[wasm_bindgen_test]
    fn decode_all_propagates_overflow() {
        // Tier-16 all-ones overflows.
        assert!(decode_all_u128(&u8s(&[0xFFu8; 17])).is_err());
    }
}
