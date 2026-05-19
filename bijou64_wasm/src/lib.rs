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
//! | JS                  | Returns                                          |
//! |---------------------|--------------------------------------------------|
//! | `encode(value)`     | `Uint8Array` (or throws `TypeError`/`RangeError`)|
//! | `decode(bytes)`     | `DecodedBijou64` (or throws `Bijou64DecodeError`)|
//! | `decodeAll(bytes)`  | `BigUint64Array` (or throws `Bijou64DecodeError`)|
//! | `encodedLen(value)` | `number` 1..=9 (or throws `TypeError`/`RangeError`) |
//! | `MAX_BYTES`         | `9` (function-style getter)                      |
//!
//! `value` is a JS `bigint` because `u64` does not fit in a `Number`.
//!
//! # Module layout
//!
//! Public Rust items live in two submodules:
//!
//! - [`encode`] — `encodedLen`, `encode`, and the
//!   [`encode::WasmBigintError`] validation type for `bigint` inputs.
//! - [`decode`] — `decode`, `decodeAll`, [`decode::WasmDecodedBijou64`],
//!   and [`decode::WasmDecodeError`] (which lowers to a JS `Error`
//!   with `name === "Bijou64DecodeError"`).
//!
//! Both submodules use `#[wasm_bindgen]` annotations directly, so the JS
//! surface is built from their contents automatically. Rust callers can
//! reach everything either via the submodule path
//! (`bijou64_wasm::encode::encode`) or via the crate-root re-exports
//! (`bijou64_wasm::encode`).
//!
//! # Bigint validation
//!
//! `encode` and `encodedLen` validate their `bigint` argument at the
//! wasm boundary. Two failure modes lower to two **native** JS error
//! types ([`js_sys::TypeError`] and [`js_sys::RangeError`]) rather than
//! plain `Error` instances with a renamed `name` property — that means
//! JS callers can use _both_ `e.name === "..."` and `e instanceof
//! TypeError` / `e instanceof RangeError` to discriminate:
//!
//! - **Non-`bigint` input** (a `Number`, `string`, `null`, `undefined`,
//!   etc.) throws a native `TypeError`. Although the Rust signature
//!   takes `&js_sys::BigInt`, wasm-bindgen does not enforce the type
//!   at runtime for `extern`-declared types; the JS shim accepts any
//!   value and we must validate ourselves.
//! - **`bigint` outside `[0n, 2n ** 64n)`** throws a native
//!   `RangeError`. wasm-bindgen's default `bigint → u64` marshalling
//!   silently truncates via `BigInt.asUintN(64, v)`, which would let
//!   two distinct application-level values produce the same encoded
//!   bytes — a direct violation of bijou's structural-canonicality
//!   guarantee. We refuse to participate.
//!
//! See [`encode::WasmBigintError`] for the typed Rust counterpart.
//!
//! # Decode errors
//!
//! Decode failures (`decode`, `decodeAll`) throw a JS `Error` with
//! `name === "Bijou64DecodeError"`. JS callers can discriminate via
//! `e.name === "Bijou64DecodeError"`; the name also appears in the
//! console-trace header (`Bijou64DecodeError: <message>`), keeping
//! bijou64 decode failures visually distinct from the many other
//! `"DecodeError"`-named errors in the JS ecosystem.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::missing_const_for_fn)]

extern crate alloc;

use wasm_bindgen::prelude::*;

pub mod decode;
pub mod encode;

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
