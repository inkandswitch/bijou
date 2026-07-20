//! Wasm/JavaScript bindings for the [`bijoux`] family — bijective
//! variable-length integer encodings for `u32`, `u64`, and `u128`.
//!
//! One wasm module, one npm package (`@inkandswitch/bijoux`), flat
//! width-suffixed exports: `encodeU64`, `decodeU64`, `decodeAllU64`,
//! `encodedLenU64`, `MAX_BYTES_U64()`, class `Decoded64` — plus the
//! `U32` / `U128` families. Free functions rather than classes for
//! better tree-shaking and cleaner TypeScript inference.
//!
//! Carrier types follow the width: `u32` uses JS `number`, `u64` and
//! `u128` use `bigint`. `decodeAllU64` returns a `BigUint64Array`;
//! `decodeAllU32` a `Uint32Array`; `decodeAllU128` a plain
//! `Array<bigint>` (no `BigUint128Array` exists in the web platform).

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::missing_const_for_fn)]

extern crate alloc;

pub mod bijou32;
pub mod bijou64;
pub mod bijou128;
