//! Signed variable-length integer encoding shootout.
//!
//! Compares the shipped signed format (bijoux::i64, wire format bijou64s) and rejected alternatives
//! against existing signed varints, before committing to an implementation:
//!
//! - **bijou64+zigzag** — zigzag-fold the sign into a `u64`, then reuse
//!   bijou64 unchanged. Faithful proxy for the future `bijou64s`: a fused
//!   implementation can only match or beat this composition.
//! - **bijou64+twos** — two's-complement cast (`i64 as u64`), the cheapest
//!   possible mapping. Included for the record: small negatives land in the
//!   top tier (9 bytes), so it should lose badly outside all-positive data.
//! - **mirrored** — prototype of the sign-in-tag / mirrored-tier design:
//!   negative length tags `0x00..=0x07`, direct window `[-120, +119]`
//!   biased into `0x08..=0xF7`, positive tags `0xF8..=0xFF`. Canonical by
//!   construction *and* lex-order preserving (unlike zigzag), at the cost
//!   of a 240-value 1-byte window and a per-sign offset table. Verified
//!   for round-trip, length, and lex order before any timing runs.
//! - **vu64+zigzag** — zigzag over the fastest unsigned competitor.
//! - **vu128** — native signed API (`encode_i64`; zigzag internally).
//! - **leb128** — native SLEB128 (`write::signed`).
//!
//! Distributions mirror the unsigned shootout, recentred on zero:
//!
//! - **Tiny** (±124): deltas, offsets, small diffs — the 1-byte window
//! - **Small** (±125–32 767): typical signed payload magnitudes
//! - **Medium** (±32 768–2³¹): large offsets
//! - **Large** (beyond ±2³¹): full-width values
//! - **Boundary**: zigzag preimages of bijou64 tier edges (sign alternates)
//! - **Uniform random**: unbiased full-range comparison
//!
//! Run: `cargo bench -p bijou64 --bench signed_shootout`

#![allow(
    missing_docs,
    unreachable_pub,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::missing_const_for_fn,
    clippy::panic,
    clippy::unnecessary_wraps,
    clippy::unwrap_used
)]

use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use criterion_pprof::criterion::{Output, PProfProfiler};
use rand::{Rng, SeedableRng, rngs::SmallRng};

// ---------------------------------------------------------------------------
// Sign-folding bijections
// ---------------------------------------------------------------------------

/// Zigzag: 0, -1, 1, -2, 2, … → 0, 1, 2, 3, 4, …
#[inline]
const fn zigzag(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

/// Inverse of [`zigzag`].
#[inline]
const fn unzigzag(u: u64) -> i64 {
    ((u >> 1) as i64) ^ -((u & 1) as i64)
}

// ---------------------------------------------------------------------------
// Mirrored-tier prototype (sign-in-tag)
// ---------------------------------------------------------------------------

/// Prototype of the mirrored/sign-in-tag signed format.
///
/// First-byte map:
///
/// ```text
/// 0x00..=0x07   negative tags   (0x00 = 8 payload bytes … 0x07 = 1 byte)
/// 0x08..=0xF7   direct values   value = byte - 0x80  →  [-120, +119]
/// 0xF8..=0xFF   positive tags   (0xF8 = 1 payload byte … 0xFF = 8 bytes)
/// ```
///
/// Positive tier `n` covers `120 + CUM[n-1] ..= 120 + CUM[n] - 1`, payload
/// big-endian `value - 120 - CUM[n-1]`. Negative tiers mirror downward from
/// `-121`, with payloads complemented so byte order ascends with value.
/// Canonical by construction (disjoint tiers, no overlong forms) and
/// lexicographic byte order matches numeric order.
mod mirrored {
    /// Cumulative tier offsets: `CUM[n] = Σ_{k=1..=n} 256^k`.
    const CUM: [u64; 8] = {
        let mut cum = [0u64; 8];
        let mut n = 1;
        while n < 8 {
            cum[n] = cum[n - 1] + 256u64.pow(n as u32);
            n += 1;
        }
        cum
    };

    /// All-ones mask for an `n`-byte payload.
    #[inline]
    const fn mask(n: usize) -> u64 {
        if n == 8 {
            u64::MAX
        } else {
            (1u64 << (8 * n)) - 1
        }
    }

    /// Payload byte count for a 0-based multi-byte offset `m`.
    #[inline]
    fn tier(m: u64) -> usize {
        let bits = 64 - (m | 1).leading_zeros() as usize;
        let b = bits.div_ceil(8);
        if m < CUM[b - 1] { b - 1 } else { b }
    }

    /// Deliberately *not* `#[inline]` (bijou64's encode regressed 13–101 %
    /// when inlined). Payload written via the shift + fixed 8-byte extend +
    /// `truncate` trick from bijou64's encode (OPTIMISATION.md).
    pub fn encode(v: i64, buf: &mut Vec<u8>) {
        if (-120..=119).contains(&v) {
            buf.push((v + 128) as u8);
        } else if v >= 120 {
            let m = (v - 120) as u64;
            let n = tier(m);
            let p = m - CUM[n - 1];
            let start = buf.len();
            buf.push(0xF7 + n as u8);
            buf.extend_from_slice(&(p << (8 * (8 - n))).to_be_bytes());
            buf.truncate(start + 1 + n);
        } else {
            let m = (-121 - v) as u64;
            let n = tier(m);
            let p = mask(n) - (m - CUM[n - 1]);
            let start = buf.len();
            buf.push(8 - n as u8);
            buf.extend_from_slice(&(p << (8 * (8 - n))).to_be_bytes());
            buf.truncate(start + 1 + n);
        }
    }

    /// Tiers 1..=7 cannot overflow (`CUM[6] + p < CUM[7] ≪ i64::MAX`), so
    /// only the 8-byte tier pays for range checks — mirroring bijou64,
    /// which checks overflow only in its top tier.
    #[inline]
    fn pos(n: usize, p: u64) -> Option<(i64, usize)> {
        Some((120 + (CUM[n - 1] + p) as i64, n + 1))
    }

    /// Largest 8-byte positive payload that stays within `i64::MAX`.
    const POS8_MAX_PAYLOAD: u64 = i64::MAX as u64 - 120 - CUM[7];

    /// Smallest 8-byte negative payload that stays within `i64::MIN`.
    /// (`m = CUM[7] + (u64::MAX - p)` must satisfy `m <= 2^63 - 121`.)
    const NEG8_MIN_PAYLOAD: u64 = u64::MAX - ((1u64 << 63) - 121 - CUM[7]);

    #[inline]
    fn pos8(p: u64) -> Option<(i64, usize)> {
        if p > POS8_MAX_PAYLOAD {
            return None;
        }
        Some(((CUM[7] + p) as i64 + 120, 9))
    }

    #[inline]
    fn neg(n: usize, p: u64) -> Option<(i64, usize)> {
        Some((-121 - (CUM[n - 1] + (mask(n) - p)) as i64, n + 1))
    }

    #[inline]
    fn neg8(p: u64) -> Option<(i64, usize)> {
        if p < NEG8_MIN_PAYLOAD {
            return None;
        }
        Some((-121 - ((CUM[7] + (u64::MAX - p)) as i64), 9))
    }

    /// Decode one value; `None` on underflow/overflow. Mirrors bijou64's
    /// per-arm slice-pattern dispatch (the consolidated single-loop variant
    /// was 4–7× slower there; see OPTIMISATION.md) and its `#[inline]`
    /// (worth 2.0–6.4× on decode there).
    #[inline]
    pub fn decode(bytes: &[u8]) -> Option<(i64, usize)> {
        // Single-compare fast path for the direct window, hoisted out of
        // the tag match (H2 experiment: bijou64's direct arm needs one
        // compare; a 0x08..=0xF7 range pattern inside a 17-arm match may
        // compile to a slower branch tree).
        if let Some(&b) = bytes.first()
            && b.wrapping_sub(0x08) <= 0xEF
        {
            return Some((i64::from(b) - 128, 1));
        }

        match *bytes {
            [0xF8, a, ..] => pos(1, u64::from(a)),
            [0xF9, a, b, ..] => pos(2, u64::from_be_bytes([0, 0, 0, 0, 0, 0, a, b])),
            [0xFA, a, b, c, ..] => pos(3, u64::from_be_bytes([0, 0, 0, 0, 0, a, b, c])),
            [0xFB, a, b, c, d, ..] => pos(4, u64::from_be_bytes([0, 0, 0, 0, a, b, c, d])),
            [0xFC, a, b, c, d, e, ..] => pos(5, u64::from_be_bytes([0, 0, 0, a, b, c, d, e])),
            [0xFD, a, b, c, d, e, f, ..] => pos(6, u64::from_be_bytes([0, 0, a, b, c, d, e, f])),
            [0xFE, a, b, c, d, e, f, g, ..] => pos(7, u64::from_be_bytes([0, a, b, c, d, e, f, g])),
            [0xFF, a, b, c, d, e, f, g, h, ..] => {
                pos8(u64::from_be_bytes([a, b, c, d, e, f, g, h]))
            }

            [0x07, a, ..] => neg(1, u64::from(a)),
            [0x06, a, b, ..] => neg(2, u64::from_be_bytes([0, 0, 0, 0, 0, 0, a, b])),
            [0x05, a, b, c, ..] => neg(3, u64::from_be_bytes([0, 0, 0, 0, 0, a, b, c])),
            [0x04, a, b, c, d, ..] => neg(4, u64::from_be_bytes([0, 0, 0, 0, a, b, c, d])),
            [0x03, a, b, c, d, e, ..] => neg(5, u64::from_be_bytes([0, 0, 0, a, b, c, d, e])),
            [0x02, a, b, c, d, e, f, ..] => neg(6, u64::from_be_bytes([0, 0, a, b, c, d, e, f])),
            [0x01, a, b, c, d, e, f, g, ..] => neg(7, u64::from_be_bytes([0, a, b, c, d, e, f, g])),
            [0x00, a, b, c, d, e, f, g, h, ..] => {
                neg8(u64::from_be_bytes([a, b, c, d, e, f, g, h]))
            }

            _ => None,
        }
    }

    /// Round-trip, consumed-length, and lex-order verification. Panics on
    /// any violation — run before timing so the prototype can't silently
    /// bench garbage.
    pub fn verify(sets: &[(&'static str, Vec<i64>)]) {
        let edges = [
            i64::MIN,
            i64::MIN + 1,
            -121,
            -120,
            -1,
            0,
            1,
            119,
            120,
            i64::MAX - 1,
            i64::MAX,
        ];

        let mut buf = Vec::new();
        let mut check = |v: i64| -> Vec<u8> {
            buf.clear();
            encode(v, &mut buf);
            let (decoded, consumed) =
                decode(&buf).unwrap_or_else(|| panic!("mirrored: decode failed for {v}: {buf:?}"));
            assert_eq!(decoded, v, "mirrored: round-trip mismatch");
            assert_eq!(consumed, buf.len(), "mirrored: length mismatch for {v}");
            buf.clone()
        };

        for &v in &edges {
            check(v);
        }

        for (_, values) in sets {
            let mut sorted = values.clone();
            sorted.sort_unstable();
            sorted.dedup();
            let mut previous: Option<(i64, Vec<u8>)> = None;
            for &v in &sorted {
                let encoded = check(v);
                if let Some((pv, pe)) = previous {
                    assert!(
                        pe < encoded,
                        "mirrored: lex order violated between {pv} and {v}"
                    );
                }
                previous = Some((v, encoded));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Value distributions
// ---------------------------------------------------------------------------

/// Fixed seed for reproducibility.
const SEED: u64 = 0xBEEF_CAFE_DEAD_F00D;

/// Number of values per batch (matches the unsigned shootout).
const BATCH: usize = 4096;

fn make_rng() -> SmallRng {
    SmallRng::seed_from_u64(SEED)
}

/// Returns a named set of signed value distributions.
fn distributions() -> Vec<(&'static str, Vec<i64>)> {
    let mut rng = make_rng();
    vec![
        (
            "tiny_pm124",
            (0..BATCH).map(|_| rng.gen_range(-124..=123i64)).collect(),
        ),
        (
            "small_pm32767",
            (0..BATCH)
                .map(|_| {
                    let magnitude = rng.gen_range(125..=32_767i64);
                    if rng.r#gen() { magnitude } else { -magnitude }
                })
                .collect(),
        ),
        (
            "medium_pm2e31",
            (0..BATCH)
                .map(|_| {
                    let magnitude = rng.gen_range(32_768..=i64::from(i32::MAX));
                    if rng.r#gen() { magnitude } else { -magnitude }
                })
                .collect(),
        ),
        (
            "medium_all_positive",
            (0..BATCH)
                .map(|_| rng.gen_range(32_768..=i64::from(i32::MAX)))
                .collect(),
        ),
        (
            "medium_all_negative",
            (0..BATCH)
                .map(|_| -rng.gen_range(32_768..=i64::from(i32::MAX)))
                .collect(),
        ),
        (
            "large_beyond_pm2e31",
            (0..BATCH)
                .map(|_| {
                    let magnitude = rng.gen_range(i64::from(i32::MAX) + 1..=i64::MAX);
                    if rng.r#gen() { magnitude } else { -magnitude }
                })
                .collect(),
        ),
        ("boundary", boundary_values()),
        (
            "uniform_random",
            (0..BATCH)
                .map(|_| rng.gen_range(i64::MIN..=i64::MAX))
                .collect(),
        ),
    ]
}

/// Signed values at and around every bijou64 tier boundary, obtained as
/// zigzag preimages of the unsigned boundary set. Sign alternates by
/// construction (even preimages are non-negative, odd are negative) —
/// worst case for branch predictors.
fn boundary_values() -> Vec<i64> {
    let unsigned_boundaries: &[u64] = &[
        0,
        247,
        248,
        503,
        504,
        66_039,
        66_040,
        16_843_255,
        16_843_256,
        4_311_810_551,
        4_311_810_552,
        1_103_823_438_327,
        1_103_823_438_328,
        282_578_800_148_983,
        282_578_800_148_984,
        72_340_172_838_076_919,
        72_340_172_838_076_920,
        u64::MAX,
    ];
    unsigned_boundaries
        .iter()
        .map(|&u| unzigzag(u))
        .cycle()
        .take(BATCH)
        .collect()
}

// ---------------------------------------------------------------------------
// Pre-encoding helpers for decode benchmarks
// ---------------------------------------------------------------------------

fn pre_encode_bijou_zigzag(values: &[i64]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        bijoux::i64::encode(v, &mut buf);
    }
    (buf, offsets)
}

fn pre_encode_bijou_twos(values: &[i64]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 9);
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        bijoux::u64::encode(v as u64, &mut buf);
    }
    (buf, offsets)
}

fn pre_encode_mirrored(values: &[i64]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        mirrored::encode(v, &mut buf);
    }
    (buf, offsets)
}

fn pre_encode_vu64_zigzag(values: &[i64]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        let encoded = vu64::encode(zigzag(v));
        buf.extend_from_slice(encoded.as_ref());
    }
    (buf, offsets)
}

fn pre_encode_vu128(values: &[i64]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut tmp = [0u8; 9];
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        let n = vu128::encode_i64(&mut tmp, v);
        buf.extend_from_slice(&tmp[..n]);
    }
    (buf, offsets)
}

fn pre_encode_leb128(values: &[i64]) -> (Vec<u8>, Vec<usize>) {
    let mut buf = Vec::with_capacity(values.len() * 5);
    let mut offsets = Vec::with_capacity(values.len());
    for &v in values {
        offsets.push(buf.len());
        leb128::write::signed(&mut buf, v).unwrap();
    }
    (buf, offsets)
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// One-time correctness gate + density report, shared by every bench target.
fn prologue(sets: &[(&'static str, Vec<i64>)]) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        mirrored::verify(sets);

        eprintln!("encoded bytes/value (density, not speed):");
        eprintln!("distribution        bijou+zz  bijou+twos  mirrored  vu64+zz  vu128  sleb128");
        for (name, values) in sets {
            let per_value = |total: usize| total as f64 / values.len() as f64;
            let zz = per_value(pre_encode_bijou_zigzag(values).0.len());
            let tc = per_value(pre_encode_bijou_twos(values).0.len());
            let mi = per_value(pre_encode_mirrored(values).0.len());
            let v64 = per_value(pre_encode_vu64_zigzag(values).0.len());
            let vu = per_value(pre_encode_vu128(values).0.len());
            let leb = per_value(pre_encode_leb128(values).0.len());
            eprintln!("{name:<19} {zz:>8.3} {tc:>11.3} {mi:>9.3} {v64:>8.3} {vu:>6.3} {leb:>8.3}");
        }
    });
}

fn bench_encode(c: &mut Criterion) {
    let sets = distributions();
    prologue(&sets);
    for (dist_name, values) in &sets {
        let mut group = c.benchmark_group(format!("signed_encode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        group.bench_function(BenchmarkId::new("bijou64s", ""), |b| {
            b.iter_batched(
                || Vec::with_capacity(BATCH * 9),
                |mut buf| {
                    for &v in values {
                        bijoux::i64::encode(v, &mut buf);
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("bijou64+twos", ""), |b| {
            b.iter_batched(
                || Vec::with_capacity(BATCH * 9),
                |mut buf| {
                    for &v in values {
                        bijoux::u64::encode(v as u64, &mut buf);
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("mirrored", ""), |b| {
            b.iter_batched(
                || Vec::with_capacity(BATCH * 9),
                |mut buf| {
                    for &v in values {
                        mirrored::encode(v, &mut buf);
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("vu64+zigzag", ""), |b| {
            b.iter_batched(
                || Vec::with_capacity(BATCH * 9),
                |mut buf| {
                    for &v in values {
                        let encoded = vu64::encode(zigzag(v));
                        buf.extend_from_slice(encoded.as_ref());
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("vu128", ""), |b| {
            b.iter_batched(
                || (Vec::with_capacity(BATCH * 9), [0u8; 9]),
                |(mut buf, mut tmp)| {
                    for &v in values {
                        let n = vu128::encode_i64(&mut tmp, v);
                        buf.extend_from_slice(&tmp[..n]);
                    }
                    buf
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("leb128", ""), |b| {
            b.iter_batched(
                || Vec::with_capacity(BATCH * 10),
                |mut buf| {
                    for &v in values {
                        leb128::write::signed(&mut buf, v).unwrap();
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
    let sets = distributions();
    prologue(&sets);
    for (dist_name, values) in &sets {
        let mut group = c.benchmark_group(format!("signed_decode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        let (zz_buf, zz_off) = pre_encode_bijou_zigzag(values);
        let (tc_buf, tc_off) = pre_encode_bijou_twos(values);
        let (mi_buf, mi_off) = pre_encode_mirrored(values);
        let (vu64_buf, vu64_off) = pre_encode_vu64_zigzag(values);
        let (vu_buf, vu_off) = pre_encode_vu128(values);
        let (leb_buf, leb_off) = pre_encode_leb128(values);

        group.bench_function(BenchmarkId::new("bijou64s", ""), |b| {
            b.iter(|| {
                let mut sum = 0i64;
                for &off in &zz_off {
                    let (v, _) = bijoux::i64::decode(&zz_buf[off..]).unwrap();
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("bijou64+twos", ""), |b| {
            b.iter(|| {
                let mut sum = 0i64;
                for &off in &tc_off {
                    let (u, _) = bijoux::u64::decode(&tc_buf[off..]).unwrap();
                    sum = sum.wrapping_add(u as i64);
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("mirrored", ""), |b| {
            b.iter(|| {
                let mut sum = 0i64;
                for &off in &mi_off {
                    let (v, _) = mirrored::decode(&mi_buf[off..]).unwrap();
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("vu64+zigzag", ""), |b| {
            b.iter(|| {
                let mut sum = 0i64;
                for &off in &vu64_off {
                    let u = vu64::decode(&vu64_buf[off..]).unwrap();
                    sum = sum.wrapping_add(unzigzag(u));
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("vu128", ""), |b| {
            b.iter(|| {
                let mut sum = 0i64;
                for &off in &vu_off {
                    // vu128 requires a &[u8; 9] — copy from the slice.
                    // In practice callers would have a buffer already;
                    // we include the copy to be fair (matches the
                    // unsigned shootout).
                    let remaining = &vu_buf[off..];
                    let mut tmp = [0u8; 9];
                    let copy_len = remaining.len().min(9);
                    tmp[..copy_len].copy_from_slice(&remaining[..copy_len]);
                    let (v, _) = vu128::decode_i64(&tmp);
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("leb128", ""), |b| {
            b.iter(|| {
                let mut sum = 0i64;
                for &off in &leb_off {
                    let mut cursor = &leb_buf[off..];
                    let v = leb128::read::signed(&mut cursor).unwrap();
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.finish();
    }
}

fn bench_stream_decode(c: &mut Criterion) {
    let sets = distributions();
    prologue(&sets);
    for (dist_name, values) in &sets {
        let mut group = c.benchmark_group(format!("signed_stream_decode/{dist_name}"));
        group.throughput(Throughput::Elements(BATCH as u64));

        let (zz_buf, _) = pre_encode_bijou_zigzag(values);
        let (tc_buf, _) = pre_encode_bijou_twos(values);
        let (mi_buf, _) = pre_encode_mirrored(values);
        let (vu64_buf, _) = pre_encode_vu64_zigzag(values);
        let (vu_buf, _) = pre_encode_vu128(values);
        let (leb_buf, _) = pre_encode_leb128(values);

        group.bench_function(BenchmarkId::new("bijou64s", ""), |b| {
            b.iter(|| {
                let mut pos = 0;
                let mut sum = 0i64;
                while pos < zz_buf.len() {
                    let (v, n) = bijoux::i64::decode(&zz_buf[pos..]).unwrap();
                    sum = sum.wrapping_add(v);
                    pos += n;
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("bijou64+twos", ""), |b| {
            b.iter(|| {
                let mut pos = 0;
                let mut sum = 0i64;
                while pos < tc_buf.len() {
                    let (u, n) = bijoux::u64::decode(&tc_buf[pos..]).unwrap();
                    sum = sum.wrapping_add(u as i64);
                    pos += n;
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("mirrored", ""), |b| {
            b.iter(|| {
                let mut pos = 0;
                let mut sum = 0i64;
                while pos < mi_buf.len() {
                    let (v, n) = mirrored::decode(&mi_buf[pos..]).unwrap();
                    sum = sum.wrapping_add(v);
                    pos += n;
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("vu64+zigzag", ""), |b| {
            b.iter(|| {
                let mut pos = 0;
                let mut sum = 0i64;
                while pos < vu64_buf.len() {
                    let n = vu64::decoded_len(vu64_buf[pos]);
                    let u = vu64::decode(&vu64_buf[pos..]).unwrap();
                    sum = sum.wrapping_add(unzigzag(u));
                    pos += n as usize;
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("vu128", ""), |b| {
            b.iter(|| {
                let mut pos = 0;
                let mut sum = 0i64;
                while pos < vu_buf.len() {
                    let remaining = &vu_buf[pos..];
                    let mut tmp = [0u8; 9];
                    let copy_len = remaining.len().min(9);
                    tmp[..copy_len].copy_from_slice(&remaining[..copy_len]);
                    let (v, consumed) = vu128::decode_i64(&tmp);
                    sum = sum.wrapping_add(v);
                    pos += consumed;
                }
                sum
            });
        });

        group.bench_function(BenchmarkId::new("leb128", ""), |b| {
            b.iter(|| {
                let mut cursor = leb_buf.as_slice();
                let mut sum = 0i64;
                while !cursor.is_empty() {
                    let v = leb128::read::signed(&mut cursor).unwrap();
                    sum = sum.wrapping_add(v);
                }
                sum
            });
        });

        group.finish();
    }
}

/// Encoded-size comparison (not a speed benchmark): report total bytes per
/// distribution per strategy, demonstrating the density cost of
/// two's-complement for small negatives.
fn bench_encoded_bytes(c: &mut Criterion) {
    let sets = distributions();
    prologue(&sets);
    let mut group = c.benchmark_group("signed_encoded_bytes");

    for (dist_name, values) in &sets {
        group.throughput(Throughput::Elements(BATCH as u64));

        group.bench_function(BenchmarkId::new("bijou64s", dist_name), |b| {
            b.iter(|| {
                let mut total = 0usize;
                for &v in values {
                    total += bijoux::i64::encoded_len(v);
                }
                total
            });
        });

        group.bench_function(BenchmarkId::new("bijou64+twos", dist_name), |b| {
            b.iter(|| {
                let mut total = 0usize;
                for &v in values {
                    total += bijoux::u64::encoded_len(v as u64);
                }
                total
            });
        });

        group.bench_function(BenchmarkId::new("vu64+zigzag", dist_name), |b| {
            b.iter(|| {
                let mut total = 0usize;
                for &v in values {
                    total += vu64::encoded_len(zigzag(v)) as usize;
                }
                total
            });
        });
    }

    group.finish();
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
        bench_stream_decode,
        bench_encoded_bytes,
}
criterion_main!(benches);
