//! _Bijoux_, plural of bijou — the bijou family of bijective,
//! length-prefixed variable-length integer encodings.
//!
//! Each format module ([`u32`](crate::u32), [`u64`](crate::u64), [`u128`](crate::u128), and in
//! future the signed `bijou64s`, …) defines **one canonical encoding
//! for one integer type**, exposed as free functions and gated behind a
//! width feature (`u32` / `u64` / `u128`, all enabled by default). The
//! [`Encode`] / [`Decode`] traits are implemented directly on the
//! integer types — the family's one-format-per-type commitment is what
//! makes `impl Encode for u64` unambiguous.
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

mod codec;

pub use codec::{Decode, Encode};
