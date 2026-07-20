//! Wasm/JavaScript bindings for [`bijou128`].
//!
//! `bijou128` is a small, stateless library: every public item operates on
//! plain numbers and byte slices, all of which are `IntoWasmAbi` in the
//! `wasm-bindgen` sense. That means we can expose the API as a flat set of
//! free functions instead of wrapping it behind a JS class — which is the
//! more idiomatic shape for utility libraries in 2026 (better tree-shaking,
//! cleaner TypeScript inference, no fake namespacing).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::missing_const_for_fn)]

extern crate alloc;

use wasm_bindgen::prelude::*;

pub mod decode;
pub mod encode;

/// Maximum number of bytes a `bijou128` encoding can occupy.
///
/// Exposed as a JS function rather than a `const` because `wasm-bindgen`
/// does not generate JS bindings for top-level `const` items; calling the
/// function once and caching is cheap.
#[must_use]
#[wasm_bindgen(js_name = MAX_BYTES)]
pub fn max_bytes() -> usize {
    bijoux::bijou128::MAX_BYTES
}
