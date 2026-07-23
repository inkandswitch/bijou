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
