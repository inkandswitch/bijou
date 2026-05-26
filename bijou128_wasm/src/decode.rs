//! Decode

use alloc::string::ToString;
use bijou128::DecodeError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Decodes a `bijou128` from the front of `bytes`.
///
/// Returns a [`WasmDecoded`] carrying the value plus the
/// number of bytes consumed (so the caller can stream-decode by
/// slicing).
///
/// # Errors
///
/// Throws a JS `Error` with `name === "Bijou128DecodeError"` if `bytes`
/// is too short for the encoding indicated by its tag byte, or if a
/// tier-16 payload would overflow `u128`. See [`WasmDecodeError`].
///
/// # JS
///
/// ```js
/// import { decode } from "bijou128";
/// const { value, bytesRead } = decode(new Uint8Array([0xF1, 0x00, 0x04, 0xFF]));
/// // value === 500n, bytesRead === 3
/// ```
#[wasm_bindgen]
pub fn decode(bytes: &[u8]) -> Result<WasmDecoded, WasmDecodeError> {
    let (value, bytes_read) = bijou128::decode(bytes)?;
    Ok(WasmDecoded { value, bytes_read })
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
/// Equivalent to calling [`decode`] in a loop, advancing by
/// `bytesRead` after each call until the buffer is empty.
///
/// # Errors
///
/// Throws a JS `Error` with `name === "Bijou128DecodeError"` (same
/// shape as [`decode`]) if any element fails to decode. The
/// partial-prefix decoded so far is *not* returned — the operation
/// is all-or-nothing.
///
/// # JS
///
/// ```js
/// import { encode, decodeAll } from "bijou128";
/// const buf = new Uint8Array([...encode(42n), ...encode(500n), ...encode(65535n)]);
/// const values = decodeAll(buf);
/// // values is [42n, 500n, 65535n]
/// ```
#[wasm_bindgen(js_name = decodeAll)]
pub fn decode_all(bytes: &[u8]) -> Result<js_sys::Array, WasmDecodeError> {
    let out = js_sys::Array::new();
    for result in bijou128::decode_iter(bytes) {
        let value: u128 = result?;
        out.push(&JsValue::from(value));
    }
    Ok(out)
}

/// A successfully-decoded bijou128 value plus its byte length.
///
/// Returned by [`decode`]. Exposes `value` (the decoded `u128`, JS
/// `bigint`) and `bytesRead` (a JS `number`) as getters. We model this
/// as a Rust-exported struct rather than constructing a plain JS
/// object via [`js_sys::Object`] because the struct gives us a real
/// TypeScript type on the JS side at zero extra runtime cost.
#[wasm_bindgen(js_name = Decoded)]
#[derive(Debug, Clone)]
#[allow(missing_copy_implementations)] // intentional per the wasm-bindgen blog post
pub struct WasmDecoded {
    value: u128,
    bytes_read: usize,
}

#[wasm_bindgen(js_class = Decoded)]
impl WasmDecoded {
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
/// We wrap [`bijou128::DecodeError`] in a newtype so we can implement
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
