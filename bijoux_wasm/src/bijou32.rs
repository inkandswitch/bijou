//! Wasm/JavaScript bindings for [`bijou32`].
//!
//! `bijou32` is a small, stateless library: every public item operates on
//! plain numbers and byte slices, all of which are `IntoWasmAbi` in the
//! `wasm-bindgen` sense. That means we can expose the API as a flat set of
//! free functions instead of wrapping it behind a JS class — which is the
//! more idiomatic shape for utility libraries in 2026 (better tree-shaking,
//! cleaner TypeScript inference, no fake namespacing).
//!
//! Unlike `bijou64_wasm` and `bijou128_wasm`, `u32` fits inside the
//! JS-safe-integer range (`[0, 2**53)`), so the wasm boundary uses plain
//! JS `number` values rather than `bigint` — no `BigInt`-shimming, no
//! out-of-range checks against `2**53`, no surprise truncation.

use wasm_bindgen::prelude::*;

pub mod decode;
pub mod encode;

/// Maximum number of bytes a `bijou32` encoding can occupy.
///
/// Exposed as a JS function rather than a `const` because `wasm-bindgen`
/// does not generate JS bindings for top-level `const` items; calling the
/// function once and caching is cheap.
#[must_use]
#[wasm_bindgen(js_name = MAX_BYTES_U32)]
pub fn max_bytes_u32() -> usize {
    bijoux::bijou32::MAX_BYTES
}
