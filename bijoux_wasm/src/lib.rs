//! Wasm/JavaScript bindings for the [`bijoux`] family — bijective
//! variable-length integer encodings, unsigned (`u32`/`u64`/`u128`)
//! and signed (`i32`/`i64`/`i128`, zigzag over the unsigned tiers).
//!
//! One wasm module, one npm package (`@inkandswitch/bijoux`), flat
//! width-suffixed exports: `encodeU64`, `decodeU64`, `decodeAllU64`,
//! `encodedLenU64`, `MAX_BYTES_U64()`, class `Decoded64` — plus the
//! `U32` / `U128` families and the signed `I32` / `I64` / `I128`
//! families (`encodeI64`, `DecodedI64`, …). Free functions rather
//! than classes for better tree-shaking and cleaner TypeScript
//! inference.
//!
//! Carrier types follow the width: 32-bit uses JS `number`, 64- and
//! 128-bit use `bigint`. `decodeAll*` returns `Uint32Array` /
//! `BigUint64Array` / `Array<bigint>` for the unsigned widths and
//! `Int32Array` / `BigInt64Array` / `Array<bigint>` for the signed
//! ones (the web platform has no 128-bit typed arrays).
//!
//! The signed formats accept the full two-sided ranges (small
//! negatives are single bytes — that's the point), and their byte
//! order is **zigzag order, not numeric order**: don't sort signed
//! encodings and expect numeric ordering.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::missing_const_for_fn)]

extern crate alloc;

pub mod i128;
pub mod i32;
pub mod i64;
pub mod u128;
pub mod u32;
pub mod u64;

/// Deliberately panics. **Not part of the public API.**
///
/// The bijoux API is total — no exported function has a reachable panic —
/// so the JS test suites need this hook to verify the `panic=unwind`
/// contract: a panic must surface as a catchable JS `Error` with
/// `name === "PanicError"`, and the Wasm instance must remain usable
/// afterward.
///
/// Gated on `debug_assertions`, so it exists only in the shipped `/debug`
/// package variant (built with the `wasm-debug` profile), never in the
/// release entry points. Both variants compile with the same wasm-bindgen
/// unwind glue, so exercising this path in the debug variant covers the
/// shared machinery.
#[cfg(debug_assertions)]
#[doc(hidden)]
#[allow(clippy::panic)] // the panic IS the feature under test
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "__triggerPanicForTesting")]
pub fn trigger_panic_for_testing() {
    panic!("deliberate test panic (__triggerPanicForTesting)");
}
