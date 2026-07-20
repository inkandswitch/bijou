//! Encode

use alloc::{string::ToString, vec::Vec};
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Failure modes for the `bigint → u64` validation that runs at the
/// wasm boundary of every function that takes a `bigint` argument
/// (currently [`encode`] and [`encoded_len`]).
///
/// Two variants, matching JS platform conventions exactly:
///
/// * [`Self::WrongType`] — the caller passed something that isn't a
///   `bigint` (a `Number`, `string`, `null`, `undefined`, etc.).
///   Although the Rust signature says `&js_sys::BigInt`, wasm-bindgen
///   does not enforce the type at runtime for `extern`-declared types;
///   the JS shim accepts any value, and we must validate ourselves.
///   Lowered to a JS native `TypeError` via [`js_sys::TypeError::new`].
///
/// * [`Self::OutOfRange`] — the caller passed a real `bigint` but its
///   value is outside `[0n, 2n ** 64n)`. wasm-bindgen's default
///   `bigint → u64` marshalling would silently truncate via
///   `BigInt.asUintN(64, v)`, which we explicitly avoid: bijou's
///   structural canonicality requires that an encoded byte sequence
///   correspond to exactly one `u64`, which is impossible if the
///   producer can feed us a `bigint` outside the range and have it
///   silently wrap. Lowered to a JS native `RangeError` via
///   [`js_sys::RangeError::new`].
///
/// Using the typed `js_sys::TypeError` / `js_sys::RangeError`
/// constructors (rather than `js_sys::Error::new(msg)` followed by
/// `set_name(...)`) means JS callers can use _both_ `e.name === "..."`
/// and `e instanceof TypeError` / `e instanceof RangeError`, because
/// the thrown values are actual platform `TypeError` / `RangeError`
/// instances.
#[derive(Debug, Clone, Copy, Error)]
pub enum WasmBigintError {
    /// Caller passed a value that isn't a JS `bigint`.
    #[error("bijou64: expected a bigint")]
    WrongType,

    /// Caller passed a `bigint` outside `[0n, 2n ** 64n)`.
    #[error("bijou64: bigint value must be in [0, 2**64)")]
    OutOfRange,
}

impl From<WasmBigintError> for JsValue {
    fn from(err: WasmBigintError) -> Self {
        match err {
            WasmBigintError::WrongType => js_sys::TypeError::new(&err.to_string()).into(),
            WasmBigintError::OutOfRange => js_sys::RangeError::new(&err.to_string()).into(),
        }
    }
}

/// Converts a `&js_sys::BigInt` to a `u64`, surfacing wrong-type and
/// out-of-range failures as typed [`WasmBigintError`] variants.
///
/// Internally the range check goes through `u64::try_from(JsValue)`,
/// which calls `__wbindgen_bigint_get_as_i64` and then verifies that
/// round-tripping the result back to `BigInt` yields the original
/// value — exactly the check we want.
fn bigint_to_u64(value: &js_sys::BigInt) -> Result<u64, WasmBigintError> {
    let js_value = JsValue::from(value.clone());

    if !js_value.is_bigint() {
        return Err(WasmBigintError::WrongType);
    }

    u64::try_from(js_value).map_err(|_| WasmBigintError::OutOfRange)
}

/// Returns the encoded length of `value` in bytes (1..=9).
///
/// # Errors
///
/// Throws a JS native [`TypeError`](js_sys::TypeError) if `value` is
/// not a `bigint`, or a JS native [`RangeError`](js_sys::RangeError)
/// if `value` is outside `[0n, 2n ** 64n)`. See [`WasmBigintError`].
///
/// # JS
///
/// ```js
/// import { encodedLen } from "bijou64";
/// encodedLen(0n);          // 1
/// encodedLen(247n);        // 1
/// encodedLen(248n);        // 2
/// encodedLen((1n << 64n) - 1n); // 9 (u64::MAX)
/// encodedLen(1n << 64n);   // throws RangeError
/// encodedLen(-1n);         // throws RangeError
/// encodedLen(42);          // throws TypeError (Number, not bigint)
/// ```
#[wasm_bindgen(js_name = encodedLen)]
pub fn encoded_len(value: &js_sys::BigInt) -> Result<usize, WasmBigintError> {
    Ok(bijoux::bijou64::encoded_len(bigint_to_u64(value)?))
}

/// Encodes `value` as a fresh `Uint8Array` (1..=9 bytes).
///
/// The Rust side allocates a `Vec<u8>` and `wasm-bindgen` copies it into
/// a JS-managed `Uint8Array` on the way out. This matches the natural
/// shape of a JS API; if you need an in-place encode into an existing
/// buffer, use the Rust crate directly.
///
/// # Errors
///
/// Throws a JS native [`TypeError`](js_sys::TypeError) if `value` is
/// not a `bigint`, or a JS native [`RangeError`](js_sys::RangeError)
/// if `value` is outside `[0n, 2n ** 64n)`. See [`WasmBigintError`].
///
/// # JS
///
/// ```js
/// import { encode } from "bijou64";
/// encode(42n);       // Uint8Array([0x2A])
/// encode(300n);      // Uint8Array([0xF8, 0x34])
/// encode(1n << 64n); // throws RangeError
/// encode(-1n);       // throws RangeError
/// encode(42);        // throws TypeError (Number, not bigint)
/// ```
#[wasm_bindgen]
pub fn encode(value: &js_sys::BigInt) -> Result<Vec<u8>, WasmBigintError> {
    let v = bigint_to_u64(value)?;
    let mut buf = Vec::with_capacity(bijoux::bijou64::MAX_BYTES);
    bijoux::bijou64::encode(v, &mut buf);
    Ok(buf)
}
