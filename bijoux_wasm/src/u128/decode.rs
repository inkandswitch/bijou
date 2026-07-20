//! Decode

use alloc::{string::ToString, vec::Vec};
use bijoux::u128::DecodeError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Validates that `bytes` is a real JS `Uint8Array` and copies it into
/// an owned `Vec<u8>`.
///
/// The decode entry points take `&JsValue` rather than `&[u8]` or
/// `&js_sys::Uint8Array` so that the rejection survives bundling.
/// wasm-bindgen's `&[u8]`/`&Uint8Array` marshalling both ultimately
/// coerce the argument through `new Uint8Array(arg)` (or equivalent),
/// which **silently truncates** any array-like whose elements fall
/// outside `0..=255` — a plain JS `[1000]` becomes `[232]`. For a codec
/// whose whole point is structural canonicality, silently corrupting
/// the input bytes is exactly the footgun we must not ship, so we
/// inspect the raw value ourselves and reject anything that isn't a
/// genuine `Uint8Array`. This mirrors the wrong-type guard
/// [`super::encode`] applies to its `bigint` argument.
///
/// The public TypeScript parameter type is still `Uint8Array` (set via
/// `unchecked_param_type` on the exports below), so typed callers are
/// unaffected; this guard only changes runtime behaviour for untyped
/// JS callers who would otherwise get silent corruption.
fn bytes_from_js(bytes: &JsValue) -> Result<Vec<u8>, WasmInputError> {
    let arr = bytes
        .dyn_ref::<js_sys::Uint8Array>()
        .ok_or(WasmInputError::WrongType)?;

    Ok(arr.to_vec())
}

/// Failure mode for the input-type validation that runs at the wasm
/// boundary of every decode entry point.
///
/// Lowered to a JS native `TypeError` (via [`js_sys::TypeError::new`])
/// so callers can use both `e.name === "TypeError"` and
/// `e instanceof TypeError`.
#[derive(Debug, Clone, Copy, Error)]
pub enum WasmInputError {
    /// Caller passed a value that isn't a JS `Uint8Array`.
    #[error("bijou128: expected a Uint8Array")]
    WrongType,
}

impl From<WasmInputError> for JsValue {
    fn from(err: WasmInputError) -> Self {
        js_sys::TypeError::new(&err.to_string()).into()
    }
}

/// Decodes a `bijou128` from the front of `bytes`.
///
/// Returns a [`WasmDecoded128`] carrying the value plus the
/// number of bytes consumed (so the caller can stream-decode by
/// slicing).
///
/// # Errors
///
/// Throws a JS native `TypeError` if `bytes` is not a `Uint8Array`.
/// Throws a JS `Error` with `name === "Bijou128DecodeError"` if `bytes`
/// is too short for the encoding indicated by its tag byte, or if a
/// tier-16 payload would overflow `u128`. See [`WasmDecodeError`].
///
/// # JS
///
/// ```js
/// import { decodeU128 } from "@inkandswitch/bijoux";
/// const { value, bytesRead } = decodeU128(new Uint8Array([0xF1, 0x00, 0x04, 0xFF]));
/// // value === 500n, bytesRead === 3
/// decodeU128([0xF1, 0x00, 0x04]); // throws TypeError (plain Array, not Uint8Array)
/// ```
#[wasm_bindgen(js_name = decodeU128)]
pub fn decode_u128(
    #[wasm_bindgen(unchecked_param_type = "Uint8Array")] bytes: &JsValue,
) -> Result<WasmDecoded128, JsValue> {
    let bytes = bytes_from_js(bytes)?;
    let (value, bytes_read) = bijoux::u128::decode(&bytes).map_err(WasmDecodeError::from)?;
    Ok(WasmDecoded128 { value, bytes_read })
}

/// Decodes every `bijou128`-encoded value in `bytes`, returning them as
/// a JS `Array<bigint>`.
///
/// Unlike `bijou64_wasm::decodeAll` (which returns a `BigUint64Array`),
/// there is no `BigUint128Array` in the web platform. Returning
/// `js_sys::Array` of `bigint`s is the natural mapping for `Vec<u128>` —
/// it preserves the full 128-bit range with zero precision loss at the
/// cost of one allocation per element on the JS side.
///
/// Equivalent to calling [`decode_u128`] in a loop, advancing by
/// `bytesRead` after each call until the buffer is empty.
///
/// # Errors
///
/// Throws a JS native `TypeError` if `bytes` is not a `Uint8Array`.
/// Throws a JS `Error` with `name === "Bijou128DecodeError"` (same
/// shape as [`decode_u128`]) if any element fails to decode. The
/// partial-prefix decoded so far is *not* returned — the operation
/// is all-or-nothing.
///
/// # JS
///
/// ```js
/// import { encodeU128, decodeAllU128 } from "@inkandswitch/bijoux";
/// const buf = new Uint8Array([...encodeU128(42n), ...encodeU128(500n), ...encodeU128(65535n)]);
/// const values = decodeAllU128(buf);
/// // values is [42n, 500n, 65535n]
/// ```
#[wasm_bindgen(js_name = decodeAllU128)]
pub fn decode_all_u128(
    #[wasm_bindgen(unchecked_param_type = "Uint8Array")] bytes: &JsValue,
) -> Result<js_sys::Array, JsValue> {
    let bytes = bytes_from_js(bytes)?;
    let out = js_sys::Array::new();
    for result in bijoux::u128::decode_iter(&bytes) {
        let value: u128 = result.map_err(WasmDecodeError::from)?;
        out.push(&JsValue::from(value));
    }
    Ok(out)
}

/// A successfully-decoded bijou128 value plus its byte length.
///
/// Returned by [`decode_u128`]. Exposes `value` (the decoded `u128`, JS
/// `bigint`) and `bytesRead` (a JS `number`) as getters. We model this
/// as a Rust-exported struct rather than constructing a plain JS
/// object via [`js_sys::Object`] because the struct gives us a real
/// TypeScript type on the JS side at zero extra runtime cost.
#[wasm_bindgen(js_name = Decoded128)]
#[derive(Debug, Clone)]
#[allow(missing_copy_implementations)] // intentional per the wasm-bindgen blog post
pub struct WasmDecoded128 {
    value: u128,
    bytes_read: usize,
}

#[wasm_bindgen(js_class = Decoded128)]
impl WasmDecoded128 {
    /// The decoded value.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> u128 {
        self.value
    }

    /// Number of bytes consumed from the input slice (1..=17).
    #[must_use]
    #[wasm_bindgen(getter, js_name = bytesRead)]
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

/// Decode failure surfaced to JS as an `Error` whose `name` is
/// `"Bijou128DecodeError"`.
///
/// We wrap [`bijoux::u128::DecodeError`] in a newtype so we can implement
/// `From<…> for JsValue` without an orphan-rule headache and so that the
/// error retains its typed shape for any downstream Rust consumer that
/// re-uses this crate.
#[derive(Debug, Clone, Copy, Error)]
#[error(transparent)]
pub struct WasmDecodeError(#[from] DecodeError);

impl From<WasmDecodeError> for DecodeError {
    fn from(err: WasmDecodeError) -> Self {
        err.0
    }
}

impl From<WasmDecodeError> for JsValue {
    fn from(err: WasmDecodeError) -> Self {
        let js_err = js_sys::Error::new(&err.to_string());
        js_err.set_name("Bijou128DecodeError");
        js_err.into()
    }
}
