//! Encode

use alloc::{string::ToString, vec::Vec};
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Failure modes for the `bigint → u128` validation that runs at the
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
///   value is outside `[0n, 2n ** 128n)`. wasm-bindgen's default
///   `bigint → u128` marshalling would silently truncate via
///   `BigInt.asUintN(128, v)`, which we explicitly avoid: bijou's
///   structural canonicality requires that an encoded byte sequence
///   correspond to exactly one `u128`, which is impossible if the
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
    #[error("bijou128: expected a bigint")]
    WrongType,

    /// Caller passed a `bigint` outside `[0n, 2n ** 128n)`.
    #[error("bijou128: bigint value must be in [0, 2**128)")]
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

/// Converts a `&js_sys::BigInt` to a `u128`, surfacing wrong-type and
/// out-of-range failures as typed [`WasmBigintError`] variants.
///
/// Internally the range check goes through `u128::try_from(JsValue)`,
/// which uses wasm-bindgen's two-word bigint marshalling (lower 64 bits
/// via `__wbindgen_bigint_get_as_i64`, upper 64 bits via `>> 64n`,
/// validated to fit in 64 bits before being recombined). Any
/// out-of-range `bigint` — including negatives — fails the high-word
/// fit check and is rejected.
fn bigint_to_u128(value: &js_sys::BigInt) -> Result<u128, WasmBigintError> {
    let js_value = JsValue::from(value.clone());

    if !js_value.is_bigint() {
        return Err(WasmBigintError::WrongType);
    }

    u128::try_from(js_value).map_err(|_| WasmBigintError::OutOfRange)
}

/// Returns the encoded length of `value` in bytes (1..=17).
///
/// # Errors
///
/// Throws a JS native [`TypeError`](js_sys::TypeError) if `value` is
/// not a `bigint`, or a JS native [`RangeError`](js_sys::RangeError)
/// if `value` is outside `[0n, 2n ** 128n)`. See [`WasmBigintError`].
///
/// # JS
///
/// ```js
/// import { encodedLen } from "bijou128";
/// encodedLen(0n);          // 1
/// encodedLen(239n);        // 1
/// encodedLen(240n);        // 2
/// encodedLen((1n << 128n) - 1n); // 17 (u128::MAX)
/// encodedLen(1n << 128n);  // throws RangeError
/// encodedLen(-1n);         // throws RangeError
/// encodedLen(42);          // throws TypeError (Number, not bigint)
/// ```
#[wasm_bindgen(js_name = encodedLen)]
pub fn encoded_len(value: &js_sys::BigInt) -> Result<usize, WasmBigintError> {
    Ok(bijoux::bijou128::encoded_len(bigint_to_u128(value)?))
}

/// Encodes `value` as a fresh `Uint8Array` (1..=17 bytes).
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
/// if `value` is outside `[0n, 2n ** 128n)`. See [`WasmBigintError`].
///
/// # JS
///
/// ```js
/// import { encode } from "bijou128";
/// encode(42n);        // Uint8Array([0x2A])
/// encode(500n);       // Uint8Array([0xF1, 0x00, 0x04])
/// encode(1n << 128n); // throws RangeError
/// encode(-1n);        // throws RangeError
/// encode(42);         // throws TypeError (Number, not bigint)
/// ```
#[wasm_bindgen]
pub fn encode(value: &js_sys::BigInt) -> Result<Vec<u8>, WasmBigintError> {
    let v = bigint_to_u128(value)?;
    let mut buf = Vec::with_capacity(bijoux::bijou128::MAX_BYTES);
    bijoux::bijou128::encode(v, &mut buf);
    Ok(buf)
}
