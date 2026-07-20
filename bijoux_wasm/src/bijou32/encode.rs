//! Encode

use alloc::{string::ToString, vec::Vec};
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Failure modes for the `number → u32` validation that runs at the
/// wasm boundary of every function that takes a numeric argument
/// (currently [`encode_u32`] and [`encoded_len_u32`]).
///
/// Two variants, matching JS platform conventions exactly:
///
/// * [`Self::WrongType`] — the caller passed something that isn't a
///   finite, integer-valued `Number` (a `NaN`, `Infinity`, fractional
///   value, `bigint`, `string`, `null`, `undefined`, etc.). Lowered to
///   a JS native `TypeError` via [`js_sys::TypeError::new`].
///
/// * [`Self::OutOfRange`] — the caller passed a valid integer Number
///   but its value is outside `[0, 2**32)`. Without this check,
///   wasm-bindgen would silently truncate via `value >>> 0` (JS
///   unsigned-32 cast), which means `-1` would silently encode as
///   `u32::MAX` and `2**32` as `0` — a footgun for content-addressed
///   protocols. Lowered to a JS native `RangeError` via
///   [`js_sys::RangeError::new`].
///
/// Using the typed `js_sys::TypeError` / `js_sys::RangeError`
/// constructors (rather than `js_sys::Error::new(msg)` followed by
/// `set_name(...)`) means JS callers can use _both_ `e.name === "..."`
/// and `e instanceof TypeError` / `e instanceof RangeError`, because
/// the thrown values are actual platform `TypeError` / `RangeError`
/// instances.
///
/// Unlike `bijou64_wasm` and `bijou128_wasm`, the input type at the JS
/// boundary is `number` (not `bigint`) because `u32::MAX` fits inside
/// `Number.MAX_SAFE_INTEGER`. `bigint` is intentionally rejected as a
/// `TypeError` to keep the API surface explicit — if you need 64-bit
/// values, use `bijou64`.
#[derive(Debug, Clone, Copy, Error)]
pub enum WasmNumberError {
    /// Caller passed a value that isn't a finite, integer-valued
    /// `Number` (NaN, Infinity, fractional, or wrong type).
    #[error("bijou32: expected a finite integer Number")]
    WrongType,

    /// Caller passed an integer Number outside `[0, 2**32)`.
    #[error("bijou32: number value must be in [0, 2**32)")]
    OutOfRange,
}

impl From<WasmNumberError> for JsValue {
    fn from(err: WasmNumberError) -> Self {
        match err {
            WasmNumberError::WrongType => js_sys::TypeError::new(&err.to_string()).into(),
            WasmNumberError::OutOfRange => js_sys::RangeError::new(&err.to_string()).into(),
        }
    }
}

/// Converts a `JsValue` to a `u32`, surfacing wrong-type and
/// out-of-range failures as typed [`WasmNumberError`] variants.
///
/// Accepts only finite, integer-valued JS `Number`s in `[0, 2**32)`.
/// Rejects:
///
/// - non-`Number` values (`bigint`, `string`, `null`, `undefined`, ...)
///   → `TypeError`
/// - `NaN`, `±Infinity`, fractional values → `TypeError`
/// - negative numbers and numbers `>= 2**32` → `RangeError`
fn jsvalue_to_u32(value: &JsValue) -> Result<u32, WasmNumberError> {
    let n = value.as_f64().ok_or(WasmNumberError::WrongType)?;

    if !n.is_finite() || n.fract() != 0.0 {
        return Err(WasmNumberError::WrongType);
    }

    if !(0.0..4_294_967_296.0).contains(&n) {
        return Err(WasmNumberError::OutOfRange);
    }

    // Safe: `n` is finite, integer-valued, in `[0, 2**32)`. The `as`
    // cast cannot truncate or lose sign because we already checked.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(n as u32)
}

/// Returns the encoded length of `value` in bytes (1..=5).
///
/// # Errors
///
/// Throws a JS native [`TypeError`](js_sys::TypeError) if `value` is
/// not a finite integer Number, or a JS native
/// [`RangeError`](js_sys::RangeError) if `value` is outside `[0, 2**32)`.
/// See [`WasmNumberError`].
///
/// # JS
///
/// ```js
/// import { encodedLenU32 } from "@inkandswitch/bijoux";
/// encodedLenU32(0);          // 1
/// encodedLenU32(251);        // 1
/// encodedLenU32(252);        // 2
/// encodedLenU32(2**32 - 1);  // 5 (u32::MAX)
/// encodedLenU32(2**32);      // throws RangeError
/// encodedLenU32(-1);         // throws RangeError
/// encodedLenU32(1.5);        // throws TypeError (not an integer)
/// encodedLenU32(42n);        // throws TypeError (bigint, not number)
/// ```
#[wasm_bindgen(js_name = encodedLenU32)]
pub fn encoded_len_u32(value: &JsValue) -> Result<usize, WasmNumberError> {
    Ok(bijoux::bijou32::encoded_len(jsvalue_to_u32(value)?))
}

/// Encodes `value` as a fresh `Uint8Array` (1..=5 bytes).
///
/// The Rust side allocates a `Vec<u8>` and `wasm-bindgen` copies it into
/// a JS-managed `Uint8Array` on the way out. This matches the natural
/// shape of a JS API; if you need an in-place encode into an existing
/// buffer, use the Rust crate directly.
///
/// # Errors
///
/// Throws a JS native [`TypeError`](js_sys::TypeError) if `value` is
/// not a finite integer Number, or a JS native
/// [`RangeError`](js_sys::RangeError) if `value` is outside `[0, 2**32)`.
/// See [`WasmNumberError`].
///
/// # JS
///
/// ```js
/// import { encodeU32 } from "@inkandswitch/bijoux";
/// encodeU32(42);          // Uint8Array([0x2A])
/// encodeU32(300);         // Uint8Array([0xFC, 0x30])
/// encodeU32(2**32);       // throws RangeError
/// encodeU32(-1);          // throws RangeError
/// encodeU32(1.5);         // throws TypeError (not an integer)
/// ```
#[wasm_bindgen(js_name = encodeU32)]
pub fn encode_u32(value: &JsValue) -> Result<Vec<u8>, WasmNumberError> {
    let v = jsvalue_to_u32(value)?;
    let mut buf = Vec::with_capacity(bijoux::bijou32::MAX_BYTES);
    bijoux::bijou32::encode(v, &mut buf);
    Ok(buf)
}
