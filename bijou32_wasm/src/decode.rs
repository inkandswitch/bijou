//! Decode

use alloc::{string::ToString, vec::Vec};
use bijou32::DecodeError;
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
/// [`crate::encode`] applies to its numeric argument.
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
    #[error("bijou32: expected a Uint8Array")]
    WrongType,
}

impl From<WasmInputError> for JsValue {
    fn from(err: WasmInputError) -> Self {
        js_sys::TypeError::new(&err.to_string()).into()
    }
}

/// Decodes a `bijou32` from the front of `bytes`.
///
/// Returns a [`WasmDecoded`] carrying the value plus the
/// number of bytes consumed (so the caller can stream-decode by
/// slicing).
///
/// # Errors
///
/// Throws a JS native `TypeError` if `bytes` is not a `Uint8Array`.
/// Throws a JS `Error` with `name === "Bijou32DecodeError"` if `bytes`
/// is too short for the encoding indicated by its tag byte, or if a
/// tier-4 payload would overflow `u32`. See [`WasmDecodeError`].
///
/// # JS
///
/// ```js
/// import { decode } from "bijou32";
/// const { value, bytesRead } = decode(new Uint8Array([0xFC, 0x30, 0xFF]));
/// // value === 300, bytesRead === 2
/// decode([0xFC, 0x30]); // throws TypeError (plain Array, not Uint8Array)
/// ```
#[wasm_bindgen]
pub fn decode(
    #[wasm_bindgen(unchecked_param_type = "Uint8Array")] bytes: &JsValue,
) -> Result<WasmDecoded, JsValue> {
    let bytes = bytes_from_js(bytes)?;
    let (value, bytes_read) = bijou32::decode(&bytes).map_err(WasmDecodeError::from)?;
    Ok(WasmDecoded { value, bytes_read })
}

/// Decodes every `bijou32`-encoded value in `bytes`, returning them as
/// a `Uint32Array`.
///
/// Equivalent to calling [`decode`] in a loop, advancing by
/// `bytesRead` after each call until the buffer is empty. Returning
/// `Vec<u32>` lets wasm-bindgen marshal the result as a typed
/// `Uint32Array`, which is denser and faster than a JS array of
/// `Number`s for large batches.
///
/// # Errors
///
/// Throws a JS native `TypeError` if `bytes` is not a `Uint8Array`.
/// Throws a JS `Error` with `name === "Bijou32DecodeError"` (same
/// shape as [`decode`]) if any element fails to decode. The
/// partial-prefix decoded so far is *not* returned — the operation
/// is all-or-nothing.
///
/// # JS
///
/// ```js
/// import { encode, decodeAll } from "bijou32";
/// const buf = new Uint8Array([...encode(42), ...encode(300), ...encode(65535)]);
/// const values = decodeAll(buf);
/// // values is Uint32Array([42, 300, 65535])
/// ```
#[wasm_bindgen(js_name = decodeAll)]
pub fn decode_all(
    #[wasm_bindgen(unchecked_param_type = "Uint8Array")] bytes: &JsValue,
) -> Result<Vec<u32>, JsValue> {
    let bytes = bytes_from_js(bytes)?;
    let mut out = Vec::new();
    for result in bijou32::decode_iter(&bytes) {
        out.push(result.map_err(WasmDecodeError::from)?);
    }
    Ok(out)
}

/// A successfully-decoded bijou32 value plus its byte length.
///
/// Returned by [`decode`]. Exposes `value` (the decoded `u32`, JS
/// `number`) and `bytesRead` (a JS `number`) as getters. We model this
/// as a Rust-exported struct rather than constructing a plain JS
/// object via [`js_sys::Object`] because the struct gives us a real
/// TypeScript type on the JS side at zero extra runtime cost.
#[wasm_bindgen(js_name = Decoded)]
#[derive(Debug, Clone, Copy)]
pub struct WasmDecoded {
    value: u32,
    bytes_read: usize,
}

#[wasm_bindgen(js_class = Decoded)]
impl WasmDecoded {
    /// The decoded value.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> u32 {
        self.value
    }

    /// Number of bytes consumed from the input slice (1..=5).
    #[must_use]
    #[wasm_bindgen(getter, js_name = bytesRead)]
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

/// Decode failure surfaced to JS as an `Error` whose `name` is
/// `"Bijou32DecodeError"`.
///
/// We wrap [`bijou32::DecodeError`] in a newtype so we can implement
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
        js_err.set_name("Bijou32DecodeError");
        js_err.into()
    }
}
