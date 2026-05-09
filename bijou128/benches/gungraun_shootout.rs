//! Deterministic instruction-count benchmarks for bijou128 using
//! [gungraun] (formerly `iai-callgrind`).
//!
//! Measures CPU instructions, cache misses, and branch mispredictions via
//! Valgrind's Callgrind. Unlike wall-clock benchmarks, results are
//! _deterministic_ -- unaffected by system load, thermal throttling, or
//! scheduling noise. Ideal for CI regression detection.
//!
//! **Requires Linux with Valgrind installed.** Will not run on macOS or Windows.
//!
//! Run:
//!   cargo bench -p bijou128 --bench gungraun_shootout
//!
//! Install runner (one-time):
//!   cargo install gungraun-runner
//!
//! [gungraun]: https://github.com/gungraun/gungraun

#![allow(
    missing_docs,
    unreachable_pub,
    clippy::doc_markdown,
    clippy::indexing_slicing,
    // gungraun's `#[benches::dist(expr, ...)]` passes owned `Vec`s from the
    // setup expressions; the bench fns iterate by reference internally but
    // must accept by value to match the macro's call shape.
    clippy::needless_pass_by_value,
    clippy::unwrap_used
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use rand::{Rng, SeedableRng, rngs::SmallRng};

// ---------------------------------------------------------------------------
// Value distributions (mirroring shootout.rs)
// ---------------------------------------------------------------------------

const BATCH: usize = 4096;
const SEED: u64 = 0xBEEF_CAFE_DEAD_F00D;

/// Per-tier offsets for bijou128 (0 plus tiers 1–16). Used to generate
/// boundary values; mirrors the private `OFFSETS` table in `bijou128`.
const OFFSETS: [u128; 17] = [
    0,
    0xF0,
    0x1F0,
    0x1_01F0,
    0x101_01F0,
    0x1_0101_01F0,
    0x101_0101_01F0,
    0x1_0101_0101_01F0,
    0x101_0101_0101_01F0,
    0x1_0101_0101_0101_01F0,
    0x101_0101_0101_0101_01F0,
    0x1_0101_0101_0101_0101_01F0,
    0x101_0101_0101_0101_0101_01F0,
    0x1_0101_0101_0101_0101_0101_01F0,
    0x101_0101_0101_0101_0101_0101_01F0,
    0x1_0101_0101_0101_0101_0101_0101_01F0,
    0x101_0101_0101_0101_0101_0101_0101_01F0,
];

fn tiny_values() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..=239u128)).collect()
}

fn small_values() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| rng.gen_range(240..=65_535u128))
        .collect()
}

fn medium_values() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| rng.gen_range(65_536..=u128::from(u32::MAX)))
        .collect()
}

fn large_u64_values() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| rng.gen_range(u128::from(u32::MAX) + 1..=u128::from(u64::MAX)))
        .collect()
}

fn xlarge_values() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| rng.gen_range(u128::from(u64::MAX) + 1..=u128::MAX))
        .collect()
}

fn uniform_values() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..=u128::MAX)).collect()
}

fn boundary_values() -> Vec<u128> {
    let mut boundaries: Vec<u128> = Vec::new();
    boundaries.push(0);
    boundaries.push(OFFSETS[1] - 1);
    for tier in 1..=16 {
        boundaries.push(OFFSETS[tier]);
        if tier < 16 {
            boundaries.push(OFFSETS[tier + 1] - 1);
        } else {
            boundaries.push(u128::MAX);
        }
    }
    boundaries.iter().copied().cycle().take(BATCH).collect()
}

// ---------------------------------------------------------------------------
// Pre-encode helpers for decode benchmarks
// ---------------------------------------------------------------------------

fn pre_encode_bijou128(values: &[u128]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * 5);
    for &v in values {
        bijou128::encode(v, &mut buf);
    }
    buf
}

fn pre_encode_vu128(values: &[u128]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut tmp = [0u8; 17];
    for &v in values {
        let n = vu128::encode_u128(&mut tmp, v);
        buf.extend_from_slice(&tmp[..n]);
    }
    buf
}

// ---------------------------------------------------------------------------
// Encode benchmarks
// ---------------------------------------------------------------------------

#[library_benchmark]
#[benches::dist(
    tiny_values(),
    small_values(),
    medium_values(),
    large_u64_values(),
    xlarge_values(),
    boundary_values(),
    uniform_values()
)]
fn encode_bijou128(values: Vec<u128>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * 17);
    for &v in black_box(&values) {
        bijou128::encode(v, &mut buf);
    }
    buf
}

#[library_benchmark]
#[benches::dist(
    tiny_values(),
    small_values(),
    medium_values(),
    large_u64_values(),
    xlarge_values(),
    boundary_values(),
    uniform_values()
)]
fn encode_vu128(values: Vec<u128>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * 17);
    let mut tmp = [0u8; 17];
    for &v in black_box(&values) {
        let n = vu128::encode_u128(&mut tmp, v);
        buf.extend_from_slice(&tmp[..n]);
    }
    buf
}

// ---------------------------------------------------------------------------
// Decode benchmarks (stream decode from concatenated buffer)
// ---------------------------------------------------------------------------

#[library_benchmark]
#[benches::dist(
    pre_encode_bijou128(&tiny_values()),
    pre_encode_bijou128(&small_values()),
    pre_encode_bijou128(&medium_values()),
    pre_encode_bijou128(&large_u64_values()),
    pre_encode_bijou128(&xlarge_values()),
    pre_encode_bijou128(&boundary_values()),
    pre_encode_bijou128(&uniform_values())
)]
fn decode_bijou128(buf: Vec<u8>) -> u128 {
    let buf = black_box(&buf);
    let mut pos = 0;
    let mut sum = 0u128;
    while pos < buf.len() {
        let (v, n) = bijou128::decode(&buf[pos..]).unwrap();
        sum = sum.wrapping_add(v);
        pos += n;
    }
    sum
}

// vu128 stream decode requires a fixed `&[u8; 19]` per call, so we copy
// each window. We decode by tracking the prefix-byte length and stepping
// through the buffer, replicating what a stream consumer would do.
#[library_benchmark]
#[benches::dist(
    pre_encode_vu128(&tiny_values()),
    pre_encode_vu128(&small_values()),
    pre_encode_vu128(&medium_values()),
    pre_encode_vu128(&large_u64_values()),
    pre_encode_vu128(&xlarge_values()),
    pre_encode_vu128(&boundary_values()),
    pre_encode_vu128(&uniform_values())
)]
fn decode_vu128(buf: Vec<u8>) -> u128 {
    let buf = black_box(&buf);
    let mut pos = 0;
    let mut sum = 0u128;
    let mut tmp = [0u8; 17];
    while pos < buf.len() {
        let remaining = &buf[pos..];
        let copy_len = remaining.len().min(17);
        tmp[..copy_len].copy_from_slice(&remaining[..copy_len]);
        // zero out any tail bytes left from a previous longer copy
        for slot in &mut tmp[copy_len..] {
            *slot = 0;
        }
        let (v, n) = vu128::decode_u128(&tmp);
        sum = sum.wrapping_add(v);
        pos += n;
    }
    sum
}

// ---------------------------------------------------------------------------
// Encoded size benchmarks
// ---------------------------------------------------------------------------

#[library_benchmark]
#[benches::dist(
    tiny_values(),
    small_values(),
    medium_values(),
    large_u64_values(),
    xlarge_values(),
    boundary_values(),
    uniform_values()
)]
fn encoded_size_bijou128(values: Vec<u128>) -> usize {
    let mut total = 0usize;
    for &v in black_box(&values) {
        total += bijou128::encoded_len(v);
    }
    total
}

// vu128 has no standalone `encoded_size(u128)` API, so we don't include it
// in this group.

// ---------------------------------------------------------------------------
// Groups + harness
// ---------------------------------------------------------------------------

library_benchmark_group!(
    name = encode_group;
    benchmarks = encode_bijou128, encode_vu128
);

library_benchmark_group!(
    name = decode_group;
    benchmarks = decode_bijou128, decode_vu128
);

library_benchmark_group!(
    name = encoded_size_group;
    benchmarks = encoded_size_bijou128
);

main!(
    library_benchmark_groups = encode_group,
    decode_group,
    encoded_size_group
);
