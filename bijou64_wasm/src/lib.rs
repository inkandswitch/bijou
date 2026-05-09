//! Wasm/JavaScript bindings for [`bijou64`].
//!
//! `bijou64` is a small, stateless library: every public item operates on
//! plain numbers and byte slices, all of which are [`IntoWasmAbi`] in the
//! `wasm-bindgen` sense. That means we can expose the API as a flat set of
//! free functions instead of wrapping it behind a JS class — which is the
//! more idiomatic shape for utility libraries in 2026 (better tree-shaking,
//! cleaner TypeScript inference, no fake namespacing).
//!
//! # Cheatsheet
//!
//! | JS                              | Returns                              |
//! |---------------------------------|--------------------------------------|
//! | `encode(value)`                 | `Uint8Array`                         |
//! | `decode(bytes)`                 | `DecodeResult` (or throws)           |
//! | `encodedLen(value)`             | `number` (1..=9)                     |
//! | `MAX_BYTES`                     | `9` (function-style getter)          |
//!
//! `value` is a JS `bigint` because `u64` does not fit in a `Number`.
//!
//! # Conventions
//!
//! Following the patterns in
//! <https://notes.brooklynzelenka.com/Blog/Notes-on-Writing-Wasm>:
//!
//! - Rust-exported types are prefixed `Wasm*`; their JS-side names drop the
//!   prefix via `js_name = …` / `js_class = …`.
//! - Errors implement `From<WasmDecodeError> for JsValue` via
//!   [`js_sys::Error`] so `Result<T, WasmDecodeError>` returns a real JS
//!   `Error` with a typed name (`"DecodeError"`) — useful for `instanceof`
//!   checks and stack traces in devtools.
//! - We do not derive `Copy` on exported handles. (`WasmDecodeResult` is
//!   plain data and could in principle be `Copy`, but consistency keeps the
//!   pattern muscle-memory.)
//! - `bijou64` is alloc-free at the boundary for this surface, so we never
//!   reach for `Rc<RefCell<…>>` here. If we ever expose stateful types
//!   (e.g. an incremental decoder), the interior-mutability rule kicks in
//!   and `wasm_refgen` becomes the right tool.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// `#[wasm_bindgen]` macro-expands to functions that we cannot mark `const`,
// so suppress this lint at the crate level rather than littering each item.
#![allow(clippy::missing_const_for_fn)]

extern crate alloc;

use alloc::{string::ToString, vec::Vec};
use bijou64::DecodeError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

/// Maximum number of bytes a `bijou64` encoding can occupy.
///
/// Exposed as a JS function rather than a `const` because `wasm-bindgen`
/// does not generate JS bindings for top-level `const` items; calling the
/// function once and caching is cheap.
#[must_use]
#[wasm_bindgen(js_name = MAX_BYTES)]
pub fn max_bytes() -> usize {
    bijou64::MAX_BYTES
}

/// Returns the encoded length of `value` in bytes (1..=9).
///
/// # JS
///
/// ```js
/// import { encodedLen } from "bijou64";
/// encodedLen(0n);          // 1
/// encodedLen(247n);        // 1
/// encodedLen(248n);        // 2
/// encodedLen((1n << 64n) - 1n); // 9 (u64::MAX)
/// ```
#[must_use]
#[wasm_bindgen(js_name = encodedLen)]
pub fn encoded_len(value: u64) -> usize {
    bijou64::encoded_len(value)
}

/// Encodes `value` as a fresh `Uint8Array` (1..=9 bytes).
///
/// The Rust side allocates a `Vec<u8>` and `wasm-bindgen` copies it into
/// a JS-managed `Uint8Array` on the way out. This matches the natural
/// shape of a JS API; if you need an in-place encode into an existing
/// buffer, use the Rust crate directly.
///
/// # JS
///
/// ```js
/// import { encode } from "bijou64";
/// encode(42n);   // Uint8Array([0x2A])
/// encode(300n);  // Uint8Array([0xF8, 0x34])
/// ```
#[must_use]
#[wasm_bindgen]
pub fn encode(value: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(bijou64::MAX_BYTES);
    bijou64::encode(value, &mut buf);
    buf
}

/// Decodes a `bijou64` from the front of `bytes`.
///
/// Returns a [`WasmDecodeResult`] carrying the value plus the number of
/// bytes consumed (so the caller can stream-decode by slicing).
///
/// # Errors
///
/// Throws a JS `Error` with `name === "DecodeError"` if `bytes` is too
/// short for the encoding indicated by its tag byte, or if a tier-8
/// payload would overflow `u64`.
///
/// # JS
///
/// ```js
/// import { decode } from "bijou64";
/// const { value, bytesRead } = decode(new Uint8Array([0xF8, 0x34, 0xFF]));
/// // value === 300n, bytesRead === 2
/// ```
#[wasm_bindgen]
pub fn decode(bytes: &[u8]) -> Result<WasmDecodeResult, WasmDecodeError> {
    let (value, bytes_read) = bijou64::decode(bytes)?;
    Ok(WasmDecodeResult { value, bytes_read })
}

/// The result of a successful [`decode`].
///
/// Exposes `value` (the decoded `u64`, JS `bigint`) and `bytesRead` (a JS
/// `number`) as getters. We model this as a Rust-exported struct rather
/// than constructing a plain JS object via [`js_sys::Object`] because the
/// struct gives us a real TypeScript type on the JS side at zero extra
/// runtime cost.
#[wasm_bindgen(js_name = DecodeResult)]
#[derive(Debug, Clone)]
#[allow(missing_copy_implementations)] // intentional per the wasm-bindgen blog post
pub struct WasmDecodeResult {
    value: u64,
    bytes_read: usize,
}

#[wasm_bindgen(js_class = DecodeResult)]
impl WasmDecodeResult {
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

/// Decode failure surfaced to JS as an `Error` with `name === "DecodeError"`.
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
        js_err.set_name("DecodeError");
        js_err.into()
    }
}
