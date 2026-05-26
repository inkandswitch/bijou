//! Decode

use alloc::string::ToString;
use bijou32::DecodeError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Decodes a `bijou32` from the front of `bytes`.
///
/// Returns a [`WasmDecoded`] carrying the value plus the
/// number of bytes consumed (so the caller can stream-decode by
/// slicing).
///
/// # Errors
///
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
/// ```
#[wasm_bindgen]
pub fn decode(bytes: &[u8]) -> Result<WasmDecoded, WasmDecodeError> {
    let (value, bytes_read) = bijou32::decode(bytes)?;
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
pub fn decode_all(bytes: &[u8]) -> Result<alloc::vec::Vec<u32>, WasmDecodeError> {
    let mut out = alloc::vec::Vec::new();
    for result in bijou32::decode_iter(bytes) {
        out.push(result?);
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
