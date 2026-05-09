//! Variable-length integer encoding shootout for bijou128.
//!
//! Compares bijou128 against vu128 across several u128 value distributions.
//! `vu128` is the only u128 varint library on crates.io we can compare
//! against directly — `varu64`, `vu64`, and `leb128` are u64-only.
//!
//! Distributions cover progressively wider ranges:
//!
//! - **Tiny** (0–239): single-byte tier 0 — common for blob counts, enum tags
//! - **Small** (240–65 535): tier 1–2
//! - **Medium** (65 536–u32::MAX): tier 2–4
//! - **Large** (u32::MAX+1–u64::MAX): tier 5–8 (still fits in u64)
//! - **XLarge** (u64::MAX+1–u128::MAX): tier 9–16 (true u128 territory)
//! - **Boundary**: worst-case branch-predictor stress at every tier edge
//! - **Uniform random**: unbiased full-range comparison
//!
//! Run: `cargo bench -p bijou128 --bench shootout`

#![allow(
    missing_docs,
    unreachable_pub,
    clippy::doc_markdown,
    clippy::indexing_slicing,
    clippy::unwrap_used
)]

use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use criterion_pprof::criterion::{Output, PProfProfiler};
use rand::{Rng, SeedableRng, rngs::SmallRng};

// ---------------------------------------------------------------------------
// Value distributions
// ---------------------------------------------------------------------------

/// Fixed seed for reproducibility. (u64 because `SmallRng::seed_from_u64`.)
const SEED: u64 = 0xBEEF_CAFE_DEAD_F00D;

/// Number of values per batch (large enough to amortise loop overhead,
/// small enough to stay in L1 cache for the encoded buffers).
const BATCH: usize = 4096;

/// Worst-case bytes per encoded value across both libraries
/// (bijou128 = 17, vu128 u128 max = 17). Used for `Vec::with_capacity`
/// sizing.
const MAX_ENC_BYTES: usize = 17;

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

fn make_rng() -> SmallRng {
    SmallRng::seed_from_u64(SEED)
}

/// Returns a named set of value distributions.
fn distributions() -> Vec<(&'static str, Vec<u128>)> {
    let mut rng = make_rng();
    vec![
        (
            "tiny_0_239",
            (0..BATCH).map(|_| rng.gen_range(0..=239u128)).collect(),
        ),
        (
            "small_240_65535",
            (0..BATCH)
                .map(|_| rng.gen_range(240..=65_535u128))
                .collect(),
        ),
        (
            "medium_64k_4G",
            (0..BATCH)
                .map(|_| rng.gen_range(65_536..=u128::from(u32::MAX)))
                .collect(),
        ),
        (
            "large_4G_to_u64max",
            (0..BATCH)
                .map(|_| rng.gen_range(u128::from(u32::MAX) + 1..=u128::from(u64::MAX)))
                .collect(),
        ),
        (
            "xlarge_above_u64",
            (0..BATCH)
                .map(|_| rng.gen_range(u128::from(u64::MAX) + 1..=u128::MAX))
                .collect(),
        ),
        ("boundary", boundary_values()),
        (
            "uniform_random",
            (0..BATCH).map(|_| rng.gen_range(0..=u128::MAX)).collect(),
        ),
    ]
}

/// Values at and around every bijou128 tier boundary — worst case for
/// branch predictors because the tag byte alternates between tiers.
fn boundary_values() -> Vec<u128> {
    let mut boundaries: Vec<u128> = Vec::new();
    boundaries.push(0); // tier 0 min
    boundaries.push(OFFSETS[1] - 1); // tier 0 max (239)
    for tier in 1..=16 {
        boundaries.push(OFFSETS[tier]); // first value in this tier
        if tier < 16 {
            boundaries.push(OFFSETS[tier + 1] - 1); // last value in this tier
        } else {
            boundaries.push(u128::MAX); // tier 16 extends to u128::MAX
        }
    }
    boundaries.iter().copied().cycle().take(BATCH).collect()
}

// ---------------------------------------------------------------------------
// Encoding helpers (uniform interface for each library)
// ---------------------------------------------------------------------------

/// Pre-encode a batch of values for decode benchmarks.
/// Returns (encoded_bytes, offsets_into_bytes).
fn pre_encode_bijou128(values: &[u128]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        bijou128::encode(v, &mut buf);
    }
    (buf, offsets)
}

fn pre_encode_vu128(values: &[u128]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut tmp = [0u8; 17];
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        let n = vu128::encode_u128(&mut tmp, v);
        buf.extend_from_slice(&tmp[..n]);
    }
    (buf, offsets)
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_encode(c: &mut Criterion) {
    for (dist_name, values) in &distributions() {
        let mut group = c.benchmark_group(format!("encode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        group.bench_function(BenchmarkId::new("bijou128", ""), |b| {
            b.iter_batched(
                || Vec::with_capacity(BATCH * MAX_ENC_BYTES),
                |mut buf| {
                    for &v in values {
                        bijou128::encode(v, &mut buf);
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("vu128", ""), |b| {
            b.iter_batched(
                || (Vec::with_capacity(BATCH * MAX_ENC_BYTES), [0u8; 17]),
                |(mut buf, mut tmp)| {
                    for &v in values {
                        let n = vu128::encode_u128(&mut tmp, v);
                        buf.extend_from_slice(&tmp[..n]);
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.finish();
    }
}

fn bench_decode(c: &mut Criterion) {
    for (dist_name, values) in &distributions() {
        let mut group = c.benchmark_group(format!("decode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        let (bijou_buf, bijou_off) = pre_encode_bijou128(values);
        let (vu_buf, vu_off) = pre_encode_vu128(values);

        group.bench_function(BenchmarkId::new("bijou128", ""), |b| {
            b.iter(|| {
                let mut sum = 0u128;
                for &off in &bijou_off {
                    let (v, _) = bijou128::decode(&bijou_buf[off..]).unwrap();
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("vu128", ""), |b| {
            b.iter(|| {
                let mut sum = 0u128;
                for &off in &vu_off {
                    // vu128 requires a &[u8; 17] for u128 decode — copy
                    // from the slice. In practice callers would have a
                    // buffer already; we include the copy to be fair.
                    let remaining = &vu_buf[off..];
                    let mut tmp = [0u8; 17];
                    let copy_len = remaining.len().min(17);
                    tmp[..copy_len].copy_from_slice(&remaining[..copy_len]);
                    let (v, _) = vu128::decode_u128(&tmp);
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.finish();
    }
}

fn bench_encoded_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoded_size");

    for (dist_name, values) in &distributions() {
        group.throughput(Throughput::Elements(BATCH as u64));

        group.bench_function(BenchmarkId::new("bijou128", dist_name), |b| {
            b.iter(|| {
                let mut total = 0usize;
                for &v in values {
                    total += bijou128::encoded_len(v);
                }
                total
            });
        });

        // vu128 does not expose a standalone `encoded_size(u128)` function
        // — it only has a prefix-byte decoder for length. Skip.
    }

    group.finish();
}

fn bench_stream_decode(c: &mut Criterion) {
    for (dist_name, values) in &distributions() {
        let mut group = c.benchmark_group(format!("stream_decode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        let (bijou_buf, _) = pre_encode_bijou128(values);

        group.bench_function(BenchmarkId::new("bijou128", ""), |b| {
            b.iter(|| {
                let mut pos = 0;
                let mut sum = 0u128;
                while pos < bijou_buf.len() {
                    let (v, n) = bijou128::decode(&bijou_buf[pos..]).unwrap();
                    sum = sum.wrapping_add(v);
                    pos += n;
                }
                sum
            });
        });

        // vu128 skipped: its fixed `&[u8; 19]` decode API doesn't naturally
        // support streaming from a contiguous buffer without precomputed
        // offsets.

        group.finish();
    }
}

/// Canonical decode: decode + verify that the encoding is minimal.
///
/// bijou128 is canonical by construction (disjoint tier ranges).
/// vu128 accepts overlong encodings, so we wrap it with a re-encode-and-
/// compare-length check to simulate what a canonical-aware caller would
/// need to do.
fn bench_canonical_decode(c: &mut Criterion) {
    for (dist_name, values) in &distributions() {
        let mut group = c.benchmark_group(format!("canonical_decode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        let (bijou_buf, bijou_off) = pre_encode_bijou128(values);
        let (vu_buf, vu_off) = pre_encode_vu128(values);

        // bijou128: canonical by construction — same as regular decode
        group.bench_function(BenchmarkId::new("bijou128", ""), |b| {
            b.iter(|| {
                let mut sum = 0u128;
                for &off in &bijou_off {
                    let (v, _) = bijou128::decode(&bijou_buf[off..]).unwrap();
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        // vu128: decode + re-encode + compare length
        group.bench_function(BenchmarkId::new("vu128", ""), |b| {
            b.iter(|| {
                let mut sum = 0u128;
                for &off in &vu_off {
                    let remaining = &vu_buf[off..];
                    let mut tmp = [0u8; 17];
                    let copy_len = remaining.len().min(17);
                    tmp[..copy_len].copy_from_slice(&remaining[..copy_len]);
                    let (v, consumed) = vu128::decode_u128(&tmp);
                    // Re-encode and verify length matches
                    let mut re = [0u8; 17];
                    let canonical_len = vu128::encode_u128(&mut re, v);
                    assert_eq!(consumed, canonical_len, "non-canonical vu128 encoding");
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.finish();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(200)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        bench_encode,
        bench_decode,
        bench_encoded_size,
        bench_stream_decode,
        bench_canonical_decode,
}
criterion_main!(benches);
