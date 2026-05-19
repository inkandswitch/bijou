//! Decode-side surface: `decode`, `decodeAll`, and the
//! [`WasmDecodedBijou64`] / [`WasmDecodeError`] types.
//!
//! Decode failures lower to a JS `Error` with `name === "Bijou64DecodeError"`.
//! JS callers can discriminate via `e.name === "Bijou64DecodeError"`;
//! the name is also what devtools render in the console trace header
//! (`Bijou64DecodeError: <message>`), distinguishing bijou64 decode
//! failures from the many other `"DecodeError"`-named errors in the JS
//! ecosystem (protobuf, CBOR, JSON, …) and forward-compatible with
//! future `bijou32` / `bijou128` crates each defining their own name.

use alloc::{string::ToString, vec::Vec};
use bijou64::DecodeError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Decodes a `bijou64` from the front of `bytes`.
///
/// Returns a [`WasmDecodedBijou64`] carrying the value plus the
/// number of bytes consumed (so the caller can stream-decode by
/// slicing).
///
/// # Errors
///
/// Throws a JS `Error` with `name === "Bijou64DecodeError"` if `bytes`
/// is too short for the encoding indicated by its tag byte, or if a
/// tier-8 payload would overflow `u64`. See [`WasmDecodeError`].
///
/// # JS
///
/// ```js
/// import { decode } from "bijou64";
/// const { value, bytesRead } = decode(new Uint8Array([0xF8, 0x34, 0xFF]));
/// // value === 300n, bytesRead === 2
/// ```
#[wasm_bindgen]
pub fn decode(bytes: &[u8]) -> Result<WasmDecoded, WasmDecodeError> {
    let (value, bytes_read) = bijou64::decode(bytes)?;
    Ok(WasmDecoded { value, bytes_read })
}

/// Decodes every `bijou64`-encoded value in `bytes`, returning them as
/// a `BigUint64Array`.
///
/// Equivalent to calling [`decode`] in a loop, advancing by
/// `bytesRead` after each call until the buffer is empty. Returning
/// `Vec<u64>` lets wasm-bindgen marshal the result as a typed
/// `BigUint64Array`, which is denser and faster than a JS array of
/// `bigint`s for large batches.
///
/// # Errors
///
/// Throws a JS `Error` with `name === "Bijou64DecodeError"` (same
/// shape as [`decode`]) if any element fails to decode. The
/// partial-prefix decoded so far is *not* returned — the operation
/// is all-or-nothing.
///
/// # JS
///
/// ```js
/// import { encode, decodeAll } from "bijou64";
/// const buf = new Uint8Array([...encode(42n), ...encode(300n), ...encode(65535n)]);
/// const values = decodeAll(buf);
/// // values is BigUint64Array([42n, 300n, 65535n])
/// ```
#[wasm_bindgen(js_name = decodeAll)]
pub fn decode_all(bytes: &[u8]) -> Result<Vec<u64>, WasmDecodeError> {
    let mut out = Vec::new();
    for result in bijou64::decode_iter(bytes) {
        out.push(result?);
    }
    Ok(out)
}

/// A successfully-decoded bijou64 value plus its byte length.
///
/// Returned by [`decode`]. Exposes `value` (the decoded `u64`, JS
/// `bigint`) and `bytesRead` (a JS `number`) as getters. We model this
/// as a Rust-exported struct rather than constructing a plain JS
/// object via [`js_sys::Object`] because the struct gives us a real
/// TypeScript type on the JS side at zero extra runtime cost.
#[wasm_bindgen(js_name = DecodedBijou64)]
#[derive(Debug, Clone)]
#[allow(missing_copy_implementations)] // intentional per the wasm-bindgen blog post
pub struct WasmDecoded {
    value: u64,
    bytes_read: usize,
}

#[wasm_bindgen(js_class = DecodedBijou64)]
impl WasmDecoded {
    /// The decoded value.
    #[must_use]
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Number of bytes consumed from the input slice (1..=9).
    #[must_use]
    #[wasm_bindgen(getter, js_name = bytesRead)]
    pub fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

/// Decode failure surfaced to JS as an `Error` whose `name` is
/// `"Bijou64DecodeError"`.
///
/// We wrap [`bijou64::DecodeError`] in a newtype so we can implement
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
        js_err.set_name("Bijou64DecodeError");
        js_err.into()
    }
}
