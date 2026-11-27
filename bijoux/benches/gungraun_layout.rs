//! Instruction-count benchmarks for the stack-allocated `Encoded*` types
//! (`encoded_bytes`, `len`, `Deref`, `IntoIterator`, `Eq`/`Ord`, `Hash`).
//!
//! These exist because the cost of these paths depends on the struct's
//! layout and on LLVM's store combining, which can shift between compiler
//! versions. `gungraun_shootout` measures `encode`/`decode`, which never
//! construct an `Encoded*`, so it can't catch regressions here.
//!
//! Run: `cargo bench -p bijoux --bench gungraun_layout` (Linux + Valgrind).

#![allow(
    missing_docs,
    unreachable_pub,
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    clippy::unwrap_used
)]

use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    hint::black_box,
};

use gungraun::{library_benchmark, library_benchmark_group, main};
use rand::{Rng, SeedableRng, rngs::SmallRng};

const BATCH: usize = 4096;
const SEED: u64 = 0xBEEF_CAFE_DEAD_F00D;

/// FNV-1a, so the hasher doesn't drown out the `Hash` impl being measured.
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

// --- u32 ---------------------------------------------------------------

/// Single-byte encodings only.
fn u32_tiny() -> Vec<u32> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..252u8).into()).collect()
}

/// Bit-width drawn uniformly, so every encoding length is common.
fn u32_mixed() -> Vec<u32> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| {
            let bits = rng.gen_range(0..=32u32);
            if bits == 0 {
                0
            } else {
                rng.gen_range(0..=u32::MAX >> (32 - bits))
            }
        })
        .collect()
}

/// Uniform over the full range (almost all max-length).
fn u32_max() -> Vec<u32> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..=u32::MAX)).collect()
}

fn u32_encs(values: Vec<u32>) -> Vec<bijoux::u32::EncodedU32> {
    values.into_iter().map(bijoux::u32::encoded_bytes).collect()
}

fn u32_map_and_keys(
    values: Vec<u32>,
) -> (
    BTreeMap<bijoux::u32::EncodedU32, u32>,
    Vec<bijoux::u32::EncodedU32>,
) {
    let keys = u32_encs(values);
    let map = keys.iter().zip(0u32..).map(|(k, i)| (*k, i)).collect();
    (map, keys)
}

#[library_benchmark]
#[benches::dist(u32_tiny(), u32_mixed(), u32_max())]
fn u32_len(values: Vec<u32>) -> usize {
    let mut total = 0usize;
    for &v in black_box(&values) {
        total += black_box(bijoux::u32::encoded_bytes(v)).len();
    }
    total
}

#[library_benchmark]
#[benches::dist(u32_tiny(), u32_mixed(), u32_max())]
fn u32_extend(values: Vec<u32>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * bijoux::u32::MAX_BYTES);
    for &v in black_box(&values) {
        buf.extend_from_slice(&bijoux::u32::encoded_bytes(v));
    }
    buf
}

#[library_benchmark]
#[benches::dist(u32_tiny(), u32_mixed(), u32_max())]
fn u32_iter(values: Vec<u32>) -> u64 {
    let mut acc = 0u64;
    for &v in black_box(&values) {
        for b in black_box(bijoux::u32::encoded_bytes(v)) {
            acc = acc.wrapping_add(u64::from(b));
        }
    }
    acc
}

#[library_benchmark]
#[benches::dist(u32_encs(u32_tiny()), u32_encs(u32_mixed()), u32_encs(u32_max()))]
fn u32_sort(mut encs: Vec<bijoux::u32::EncodedU32>) -> Vec<bijoux::u32::EncodedU32> {
    black_box(&mut encs).sort_unstable();
    encs
}

#[library_benchmark]
#[benches::dist(
    u32_map_and_keys(u32_tiny()),
    u32_map_and_keys(u32_mixed()),
    u32_map_and_keys(u32_max())
)]
fn u32_btree_get(
    input: (
        BTreeMap<bijoux::u32::EncodedU32, u32>,
        Vec<bijoux::u32::EncodedU32>,
    ),
) -> u64 {
    let (map, keys) = input;
    let mut acc = 0u64;
    for k in black_box(&keys) {
        acc += u64::from(*map.get(k).unwrap());
    }
    acc
}

#[library_benchmark]
#[benches::dist(u32_encs(u32_tiny()), u32_encs(u32_mixed()), u32_encs(u32_max()))]
fn u32_hash(encs: Vec<bijoux::u32::EncodedU32>) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    for e in black_box(&encs) {
        e.hash(&mut h);
    }
    h.finish()
}

library_benchmark_group!(
    name = u32_layout;
    benchmarks = u32_len, u32_extend, u32_iter, u32_sort, u32_btree_get, u32_hash
);

// --- u64 ---------------------------------------------------------------

/// Single-byte encodings only.
fn u64_tiny() -> Vec<u64> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..248u8).into()).collect()
}

/// Bit-width drawn uniformly, so every encoding length is common.
fn u64_mixed() -> Vec<u64> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| {
            let bits = rng.gen_range(0..=64u32);
            if bits == 0 {
                0
            } else {
                rng.gen_range(0..=u64::MAX >> (64 - bits))
            }
        })
        .collect()
}

/// Uniform over the full range (almost all max-length).
fn u64_max() -> Vec<u64> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..=u64::MAX)).collect()
}

fn u64_encs(values: Vec<u64>) -> Vec<bijoux::u64::EncodedU64> {
    values.into_iter().map(bijoux::u64::encoded_bytes).collect()
}

fn u64_map_and_keys(
    values: Vec<u64>,
) -> (
    BTreeMap<bijoux::u64::EncodedU64, u32>,
    Vec<bijoux::u64::EncodedU64>,
) {
    let keys = u64_encs(values);
    let map = keys.iter().zip(0u32..).map(|(k, i)| (*k, i)).collect();
    (map, keys)
}

#[library_benchmark]
#[benches::dist(u64_tiny(), u64_mixed(), u64_max())]
fn u64_len(values: Vec<u64>) -> usize {
    let mut total = 0usize;
    for &v in black_box(&values) {
        total += black_box(bijoux::u64::encoded_bytes(v)).len();
    }
    total
}

#[library_benchmark]
#[benches::dist(u64_tiny(), u64_mixed(), u64_max())]
fn u64_extend(values: Vec<u64>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * bijoux::u64::MAX_BYTES);
    for &v in black_box(&values) {
        buf.extend_from_slice(&bijoux::u64::encoded_bytes(v));
    }
    buf
}

#[library_benchmark]
#[benches::dist(u64_tiny(), u64_mixed(), u64_max())]
fn u64_iter(values: Vec<u64>) -> u64 {
    let mut acc = 0u64;
    for &v in black_box(&values) {
        for b in black_box(bijoux::u64::encoded_bytes(v)) {
            acc = acc.wrapping_add(u64::from(b));
        }
    }
    acc
}

#[library_benchmark]
#[benches::dist(u64_encs(u64_tiny()), u64_encs(u64_mixed()), u64_encs(u64_max()))]
fn u64_sort(mut encs: Vec<bijoux::u64::EncodedU64>) -> Vec<bijoux::u64::EncodedU64> {
    black_box(&mut encs).sort_unstable();
    encs
}

#[library_benchmark]
#[benches::dist(
    u64_map_and_keys(u64_tiny()),
    u64_map_and_keys(u64_mixed()),
    u64_map_and_keys(u64_max())
)]
fn u64_btree_get(
    input: (
        BTreeMap<bijoux::u64::EncodedU64, u32>,
        Vec<bijoux::u64::EncodedU64>,
    ),
) -> u64 {
    let (map, keys) = input;
    let mut acc = 0u64;
    for k in black_box(&keys) {
        acc += u64::from(*map.get(k).unwrap());
    }
    acc
}

#[library_benchmark]
#[benches::dist(u64_encs(u64_tiny()), u64_encs(u64_mixed()), u64_encs(u64_max()))]
fn u64_hash(encs: Vec<bijoux::u64::EncodedU64>) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    for e in black_box(&encs) {
        e.hash(&mut h);
    }
    h.finish()
}

library_benchmark_group!(
    name = u64_layout;
    benchmarks = u64_len, u64_extend, u64_iter, u64_sort, u64_btree_get, u64_hash
);

// --- u128 ---------------------------------------------------------------

/// Single-byte encodings only.
fn u128_tiny() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..240u8).into()).collect()
}

/// Bit-width drawn uniformly, so every encoding length is common.
fn u128_mixed() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH)
        .map(|_| {
            let bits = rng.gen_range(0..=128u32);
            if bits == 0 {
                0
            } else {
                rng.gen_range(0..=u128::MAX >> (128 - bits))
            }
        })
        .collect()
}

/// Uniform over the full range (almost all max-length).
fn u128_max() -> Vec<u128> {
    let mut rng = SmallRng::seed_from_u64(SEED);
    (0..BATCH).map(|_| rng.gen_range(0..=u128::MAX)).collect()
}

fn u128_encs(values: Vec<u128>) -> Vec<bijoux::u128::EncodedU128> {
    values
        .into_iter()
        .map(bijoux::u128::encoded_bytes)
        .collect()
}

fn u128_map_and_keys(
    values: Vec<u128>,
) -> (
    BTreeMap<bijoux::u128::EncodedU128, u32>,
    Vec<bijoux::u128::EncodedU128>,
) {
    let keys = u128_encs(values);
    let map = keys.iter().zip(0u32..).map(|(k, i)| (*k, i)).collect();
    (map, keys)
}

#[library_benchmark]
#[benches::dist(u128_tiny(), u128_mixed(), u128_max())]
fn u128_len(values: Vec<u128>) -> usize {
    let mut total = 0usize;
    for &v in black_box(&values) {
        total += black_box(bijoux::u128::encoded_bytes(v)).len();
    }
    total
}

#[library_benchmark]
#[benches::dist(u128_tiny(), u128_mixed(), u128_max())]
fn u128_extend(values: Vec<u128>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * bijoux::u128::MAX_BYTES);
    for &v in black_box(&values) {
        buf.extend_from_slice(&bijoux::u128::encoded_bytes(v));
    }
    buf
}

#[library_benchmark]
#[benches::dist(u128_tiny(), u128_mixed(), u128_max())]
fn u128_iter(values: Vec<u128>) -> u64 {
    let mut acc = 0u64;
    for &v in black_box(&values) {
        for b in black_box(bijoux::u128::encoded_bytes(v)) {
            acc = acc.wrapping_add(u64::from(b));
        }
    }
    acc
}

#[library_benchmark]
#[benches::dist(u128_encs(u128_tiny()), u128_encs(u128_mixed()), u128_encs(u128_max()))]
fn u128_sort(mut encs: Vec<bijoux::u128::EncodedU128>) -> Vec<bijoux::u128::EncodedU128> {
    black_box(&mut encs).sort_unstable();
    encs
}

#[library_benchmark]
#[benches::dist(
    u128_map_and_keys(u128_tiny()),
    u128_map_and_keys(u128_mixed()),
    u128_map_and_keys(u128_max())
)]
fn u128_btree_get(
    input: (
        BTreeMap<bijoux::u128::EncodedU128, u32>,
        Vec<bijoux::u128::EncodedU128>,
    ),
) -> u64 {
    let (map, keys) = input;
    let mut acc = 0u64;
    for k in black_box(&keys) {
        acc += u64::from(*map.get(k).unwrap());
    }
    acc
}

#[library_benchmark]
#[benches::dist(u128_encs(u128_tiny()), u128_encs(u128_mixed()), u128_encs(u128_max()))]
fn u128_hash(encs: Vec<bijoux::u128::EncodedU128>) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    for e in black_box(&encs) {
        e.hash(&mut h);
    }
    h.finish()
}

library_benchmark_group!(
    name = u128_layout;
    benchmarks = u128_len, u128_extend, u128_iter, u128_sort, u128_btree_get, u128_hash
);

main!(
    library_benchmark_groups = u32_layout,
    u64_layout,
    u128_layout
);
