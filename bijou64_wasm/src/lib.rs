//! Wasm/JavaScript bindings for [`bijou64`].
//!
//! `bijou64` is a small, stateless library: every public item operates on
//! plain numbers and byte slices, all of which are `IntoWasmAbi` in the
//! `wasm-bindgen` sense. That means we can expose the API as a flat set of
//! free functions instead of wrapping it behind a JS class — which is the
//! more idiomatic shape for utility libraries in 2026 (better tree-shaking,
//! cleaner TypeScript inference, no fake namespacing).
//!
//! # Cheatsheet
//!
//! | JS                              | Returns                              |
//! |---------------------------------|--------------------------------------|
//! | `encode(value)`                 | `Uint8Array` (or throws `RangeError`)|
//! | `decode(bytes)`                 | `DecodeResult` (or throws `DecodeError`) |
//! | `encodedLen(value)`             | `number` 1..=9 (or throws `RangeError`)  |
//! | `MAX_BYTES`                     | `9` (function-style getter)          |
//!
//! `value` is a JS `bigint` because `u64` does not fit in a `Number`.
//!
//! # Range checking
//!
//! `encode` and `encodedLen` reject any `bigint` outside `[0n, 2n ** 64n)`
//! with a JS `Error` whose `name === "RangeError"`. This is a deliberate
//! divergence from the wasm-bindgen default, which silently truncates
//! out-of-range bigints via `BigInt.asUintN(64, value)`. Silent truncation
//! would let two distinct application-level values produce the same
//! encoded bytes — a direct violation of bijou's structural-canonicality
//! guarantee. We take `&js_sys::BigInt` and validate via
//! `u64::try_from(…)` (which uses wasm-bindgen's `try_from_js_value_ref`
//! intrinsic to detect the truncation).
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
//!   checks and stack traces in devtools. The same pattern is used for the
//!   `RangeError` thrown on out-of-range `bigint` input.
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

/// Converts a `&js_sys::BigInt` to a `u64`, distinguishing two failure
/// modes that wasm-bindgen's default marshalling would conflate:
///
/// * **Wrong type** — caller passed something that isn't a `bigint`
///   (a `Number`, `string`, `null`, `undefined`, etc.). Although the
///   Rust signature says `&js_sys::BigInt`, wasm-bindgen does not
///   enforce the type at runtime for `extern`-declared types; the JS
///   shim accepts any value. We throw a JS `TypeError`.
///
/// * **Out of range** — caller passed a real `bigint` but its value
///   is outside `[0n, 2n ** 64n)`. wasm-bindgen's default `bigint → u64`
///   marshalling would silently truncate via `BigInt.asUintN(64, v)`,
///   which we explicitly avoid: bijou's structural canonicality
///   requires that an encoded byte sequence correspond to exactly one
///   `u64`, which is impossible if the producer can feed us a `bigint`
///   outside the range and have it silently wrap. We throw a JS
///   `RangeError`.
///
/// The two errors have distinct `name` properties on the JS side so
/// callers can tell them apart with `e.name === "TypeError"` /
/// `e.name === "RangeError"` or `instanceof` guards.
///
/// Internally the range check goes through `JsValue::try_from::<u64>`,
/// which calls `__wbindgen_bigint_get_as_i64` and then verifies that
/// round-tripping the result back to `BigInt` yields the original
/// value — exactly the check we want.
fn bigint_to_u64(value: &js_sys::BigInt) -> Result<u64, JsValue> {
    let js_value = JsValue::from(value.clone());

    if !js_value.is_bigint() {
        let err = js_sys::Error::new("bijou64: expected a bigint");
        err.set_name("TypeError");
        return Err(err.into());
    }

    u64::try_from(js_value).map_err(|_| {
        let err = js_sys::Error::new("bijou64: bigint value must be in [0, 2**64)");
        err.set_name("RangeError");
        err.into()
    })
}

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
/// # Errors
///
/// Throws a JS `Error` with `name === "RangeError"` if `value` is
/// outside `[0n, 2n ** 64n)`. See the module-level "Range checking"
/// section for why we reject rather than silently wrap.
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
/// ```
#[wasm_bindgen(js_name = encodedLen)]
pub fn encoded_len(value: &js_sys::BigInt) -> Result<usize, JsValue> {
    Ok(bijou64::encoded_len(bigint_to_u64(value)?))
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
/// Throws a JS `Error` with `name === "RangeError"` if `value` is
/// outside `[0n, 2n ** 64n)`.
///
/// # JS
///
/// ```js
/// import { encode } from "bijou64";
/// encode(42n);            // Uint8Array([0x2A])
/// encode(300n);           // Uint8Array([0xF8, 0x34])
/// encode(1n << 64n);      // throws RangeError
/// encode(-1n);            // throws RangeError
/// ```
#[wasm_bindgen]
pub fn encode(value: &js_sys::BigInt) -> Result<Vec<u8>, JsValue> {
    let v = bigint_to_u64(value)?;
    let mut buf = Vec::with_capacity(bijou64::MAX_BYTES);
    bijou64::encode(v, &mut buf);
    Ok(buf)
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
/// Throws a `DecodeError` (same shape as [`decode`]) if any element
/// fails to decode. The partial-prefix decoded so far is *not*
/// returned — the operation is all-or-nothing.
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
