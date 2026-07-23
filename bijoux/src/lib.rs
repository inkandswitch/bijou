//! 💎 _Bijoux_ — the bijou family of bijective, length-prefixed
//! variable-length integer encodings. (Read the name both ways:
//! **bijouX** with `X` ranging over the widths — bijou32, bijou64,
//! bijou128 — and the French plural: all of the bijous.)
//!
//! Each format module — unsigned [`u32`](mod@u32), [`u64`](mod@u64),
//! and [`u128`](mod@u128), and signed [`i32`](mod@i32),
//! [`i64`](mod@i64), and [`i128`](mod@i128) (zigzag over the matching
//! unsigned tier scheme) — defines **one canonical encoding for one
//! integer type**, exposed as free functions and gated behind a
//! same-named feature (all enabled by default). The [`Encode`] /
//! [`Decode`] traits are implemented directly on the integer types —
//! the family's one-format-per-type commitment is what makes
//! `impl Encode for u64` unambiguous.
//!
//! Byte-lexicographic order matches numeric order for the unsigned
//! formats; for the signed formats it is zigzag order (see the
//! [`i64`](mod@i64) module's ordering caveat).
//!
//! # Examples
//!
//! Method syntax on the integers themselves:
//!
//! ```
//! use bijoux::{Decode, Encode};
//!
//! let mut buf = Vec::new();
//! 300u64.encode(&mut buf);
//!
//! let (value, consumed) = u64::decode(&buf).unwrap();
//! assert_eq!((value, consumed), (300, 2));
//! ```
//!
//! Code generic over any bijou-encodable integer:
//!
//! ```
//! use bijoux::Encode;
//!
//! fn frame<T: Encode>(values: &[T], buf: &mut Vec<u8>) {
//!     for &value in values {
//!         value.encode(buf);
//!     }
//! }
//!
//! let mut buf = Vec::new();
//! frame(&[0u64, 247, 248], &mut buf);
//! assert_eq!(buf.len(), 1 + 1 + 2); // third value crosses into tier 1
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

extern crate alloc;

#[cfg(feature = "i128")]
pub mod i128;
#[cfg(feature = "i32")]
pub mod i32;
#[cfg(feature = "i64")]
pub mod i64;
#[cfg(feature = "u128")]
pub mod u128;
#[cfg(feature = "u32")]
pub mod u32;
#[cfg(feature = "u64")]
pub mod u64;

// The signed formats are implemented over the matching unsigned tier
// machinery, so a signed-only build still *compiles* the unsigned
// module — but privately: enabling `i64` without `u64` does not export
// the `u64` API (module, trait impls, or docs).
#[cfg(all(feature = "i128", not(feature = "u128")))]
mod u128;
#[cfg(all(feature = "i32", not(feature = "u32")))]
mod u32;
#[cfg(all(feature = "i64", not(feature = "u64")))]
mod u64;

mod codec;

pub use codec::{Decode, Encode};
