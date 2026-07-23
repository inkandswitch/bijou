# bijou64 Benchmark Shootout (x86)

> Criterion benchmarks comparing bijou64 against varu64, vu64, vu128, and leb128 across six value distributions over batches of 4096 values.
>
> Run: `cargo bench -p bijoux --bench shootout`
>
> See also: [ARM results (Apple M2 Pro)](SHOOTOUT_ANALYSIS_ARM.md)

## Methodology

### Wall-Clock Benchmarks (Criterion)

| Setting | Value |
|---------|-------|
| Framework | [Criterion 0.5](https://bheisler.github.io/criterion.rs/) with [pprof flamegraphs](https://docs.rs/pprof/latest/) |
| Sample size | 200 iterations per benchmark |
| Warm-up | 3 seconds |
| Measurement time | 5 seconds per benchmark |
| Batch size | 4096 values (L1-cache-friendly) |
| Seed | `0xBEEF_CAFE_DEAD_F00D` (fixed for reproducibility) |
| Profile | `bench` (`opt-level = 3`, `lto = "thin"`, `debug = true`) |

### Instruction-Count Benchmarks (gungraun)

Deterministic benchmarks via Valgrind's Callgrind. Reports CPU instructions, cache misses, and branch mispredictions. Unaffected by system load or scheduling noise -- ideal for CI regression detection.

| Setting | Value |
|---------|-------|
| Framework | [gungraun 0.18](https://github.com/gungraun/gungraun) (formerly `iai-callgrind`) |
| Platform | Linux only (requires Valgrind) |
| CI | `.github/workflows/gungraun-bench.yml` |

Run locally (Linux only):
```bash
cargo install gungraun-runner
cargo bench -p bijoux --bench gungraun_shootout
```

### Chart Generation

All charts are auto-generated from Criterion's raw sample data (`target/criterion/**/new/sample.json`).

```bash
# via nix flake app
nix run .#bench-charts

# or via uv (auto-installs Python deps)
uv run bijoux/charts/analyze.py --arch x86
```

Output (into `bijoux/charts/<arch>/`):
- `bijoux/charts/<arch>/percentiles.csv` -- machine-readable statistics
- `bijoux/charts/<arch>/percentiles.md` -- markdown tables with p50/p90/p95/p99/p99.9
- `bijoux/charts/<arch>/*_box.svg` -- box-and-whisker plots
- `bijoux/charts/<arch>/*_bar.svg` -- grouped bar charts (median + p5-p95 whiskers)
- `bijoux/charts/<arch>/*_cdf.svg` -- CDF overlay plots
- `bijoux/charts/<arch>/*_heatmap.svg` -- library x distribution heatmaps
- `bijoux/charts/<arch>/*_cdf.html` -- interactive Plotly CDFs (hover, zoom)
- `bijoux/charts/<arch>/*_heatmap.html` -- interactive Plotly heatmaps
- `bijoux/charts/<arch>/percentiles.html` -- sortable/filterable percentile table

### Value Distributions

| Name | Range | Rationale |
|------|-------|-----------|
| tiny (0-247) | Single-byte bijou64 tier | Blob counts, small lengths, enum tags |
| small (248-64k) | 248 -- 65,535 | Typical payload sizes |
| medium (64k-4B) | 65,536 -- 4,294,967,295 | Large blob sizes, offsets |
| large (>4B) | > 2^32 | Content hashes as integers, counters |
| boundary | All 18 tier-edge values, cycled | Worst-case branch prediction |
| uniform random | Full u64 range | Unbiased comparison |

## Machine

|         |                                            |
|---------|--------------------------------------------|
| CPU     | AMD Ryzen AI 9 HX 370 (Zen 5)              |
| Cores   | 12C / 24T                                  |
| L1d     | 48 KiB per core (576 KiB total)            |
| L2      | 1 MiB per core (12 MiB total)              |
| L3      | 24 MiB (2 instances)                       |
| OS      | NixOS, Linux 7.0.2                         |
| Rust    | 1.90.0                                     |
| Profile | `bench` (opt-level = 3)                    |

> All medians are taken from [`charts/x86/percentiles.md`](../charts/x86/percentiles.md).

## Encode

Encode to a `Vec<u8>`.

| Distribution    | bijou64   | varu64 | vu64  | vu128     | leb128    | bijou64 rank | bijou64 vs other best |
|-----------------|-----------|--------|-------|-----------|-----------|--------------|-----------------------|
| tiny (0-247)    | **1.92**  |  9.48  | 20.81 | 10.96     |  4.83     | #1           | 0.40x                 |
| small (248-64k) | 10.37     | 16.78  | 22.29 | 15.65     | **8.37**  | #2           | 1.24x                 |
| medium (64k-4B) | **11.02** | 41.89  | 47.24 | 30.85     | 27.08     | #1           | 0.41x                 |
| large (>4B)     | **18.95** | 46.28  | 58.48 | 30.70     | 27.31     | #1           | 0.69x                 |
| boundary        | **10.64** | 20.22  | 22.79 | 15.65     | 14.38     | #1           | 0.74x                 |
| uniform random  | **11.93** | 24.11  | 27.18 | 15.23     | 27.72     | #1           | 0.78x                 |

<details open>
<summary>Charts</summary>

![Encode — Bar Chart](../charts/x86/encode_bar.svg)
![Encode — Box Plot](../charts/x86/encode_box.svg)
![Encode — CDF](../charts/x86/encode_cdf.svg)

</details>

## Encoded Size Query (runtime)

Wall-clock time to call `encoded_len(v)` across the 6 distributions.
vu128 and leb128 are excluded: neither crate exposes a standalone
`encoded_len(u64)` query (both compute size only as a side effect of
encoding).

> For the arch-independent _format_ size comparison (how many bytes
> each format uses), see [SIZE_ANALYSIS.md](../design/SIZE_ANALYSIS.md).

| Distribution    | bijou64 | varu64 | vu64     | bijou64 rank | bijou64 vs other best |
|-----------------|--------:|-------:|---------:|--------------|-----------------------|
| tiny (0-247)    | 1.75    | **0.91** | 1.09   | #3           | 1.92x                 |
| small (248-64k) | 2.67    | 1.79   | **1.11** | #3           | 2.41x                 |
| medium (64k-4B) | 2.67    | 2.52   | **1.09** | #3           | 2.45x                 |
| large (>4B)     | 2.69    | 4.77   | **1.09** | #2           | 2.47x                 |
| boundary        | 2.44    | 3.21   | **1.09** | #2           | 2.23x                 |
| uniform random  | 2.67    | 4.77   | **1.09** | #2           | 2.44x                 |

<details open>
<summary>Charts</summary>

![Encoded Size — Bar Chart](../charts/x86/encoded_size_bar.svg)
![Encoded Size — Box Plot](../charts/x86/encoded_size_box.svg)
![Encoded Size — CDF](../charts/x86/encoded_size_cdf.svg)

</details>

vu64 wins every cell because its tier boundaries are exact powers of
2, so `encoded_len` reduces to a single `leading_zeros` instruction
with no correction step. bijou64's per-tier offsets force an extra
comparison; the gap (~2.2-2.5x) is unavoidable for the
canonicality-preserving path. See [OPTIMISATION.md](../design/OPTIMISATION.md)
for the full analysis.

## Decode

Decode from a `&[u8]` buffer.

| Distribution    | bijou64   | varu64 | vu64  | vu128 | leb128 | bijou64 rank | bijou64 vs other best |
|-----------------|-----------|--------|-------|-------|--------|--------------|-----------------------|
| tiny (0-247)    | **1.78**  |  5.71  |  8.57 |  6.84 |  3.41  | #1           | 0.52x                 |
| small (248-64k) | **3.93**  |  8.85  | 12.38 |  8.58 |  8.47  | #1           | 0.46x                 |
| medium (64k-4B) | **3.86**  | 13.01  | 15.17 |  9.62 | 11.56  | #1           | 0.40x                 |
| large (>4B)     | **3.11**  | 18.84  |  8.04 |  6.95 | 30.75  | #1           | 0.45x                 |
| boundary        | **3.75**  | 13.76  | 11.62 |  7.81 |  9.74  | #1           | 0.48x                 |
| uniform random  | **3.08**  | 18.76  |  7.98 |  7.24 | 29.89  | #1           | 0.43x                 |

<details open>
<summary>Charts</summary>

![Decode — Bar Chart](../charts/x86/decode_bar.svg)
![Decode — Box Plot](../charts/x86/decode_box.svg)
![Decode — CDF](../charts/x86/decode_cdf.svg)

</details>

bijou64 wins every decode cell on Zen 5, with margins ranging from 1.9× (tiny vs leb128) to 2.5× (medium vs vu128). gungraun reports `decode/tiny` running in ~38 000 modelled cycles per 4096 values — fewer than every competitor on every distribution.


## Canonical Decode

Decode with a guarantee that the encoding is minimal (no overlong representations accepted). This matters for protocols that need deterministic serialisation -- if two peers can encode the same value differently, content-addressed hashes break.

bijou64 achieves canonicality structurally: its disjoint tier ranges make overlong encodings impossible, so the canonical decode path is identical to regular decode with zero overhead. varu64 and vu64 always perform a runtime minimality check (there's no way to opt out). vu128 and leb128 accept overlong encodings by design, so we wrap them with a decode-then-re-encode-and-compare-length check to simulate what a canonical-aware caller would need to do.

| Distribution    | bijou64   | varu64 | vu64  | vu128 | leb128 | bijou64 rank | bijou64 vs other best |
|-----------------|-----------|--------|-------|-------|--------|--------------|-----------------------|
| tiny (0-247)    | **1.75**  |  6.84  |  8.76 | 10.37 |  7.75  | #1           | 0.26x                 |
| small (248-64k) | **4.07**  |  8.85  | 12.71 | 17.13 | 17.68  | #1           | 0.46x                 |
| medium (64k-4B) | **4.01**  | 12.14  | 15.36 | 12.33 | 23.45  | #1           | 0.33x                 |
| large (>4B)     | **3.18**  | 19.13  |  8.29 |  9.81 | 53.24  | #1           | 0.38x                 |
| boundary        | **3.77**  | 13.55  | 11.78 |  9.80 | 21.69  | #1           | 0.38x                 |
| uniform random  | **3.14**  | 19.09  |  7.96 |  9.91 | 52.41  | #1           | 0.39x                 |

<details open>
<summary>Charts</summary>

![Canonical Decode — Bar Chart](../charts/x86/canonical_decode_bar.svg)
![Canonical Decode — CDF](../charts/x86/canonical_decode_cdf.svg)

</details>

The cost of canonicality varies wildly by crate. bijou64's numbers are identical to plain decode because there's nothing extra to check. varu64 always pays its runtime check, so its column matches the plain decode table. vu128 and leb128 pay the round-trip re-encode penalty — leb128 in particular is catastrophic on large/uniform (~53 µs vs ~30 µs without the check) because of its byte-at-a-time `Write`/`Read` interface.

bijou64 wins all 6 canonical decode distributions on x86, including tiny — where structural canonicality edges out varu64's runtime check (1.75 µs vs 6.84 µs). For protocols that _require_ canonical encoding, this is the table that matters.

## Stream Decode

Decode a concatenated stream of encoded values, advancing a cursor by the number of bytes consumed per call. vu128 is included with the per-iteration `[u8; 9]` copy required by its API (matching the treatment in plain decode and canonical decode — see [Methodology / vu128 copy cost](#vu128-copy-cost) below).

| Distribution    | bijou64   | varu64 | vu64  | vu128 | leb128 | bijou64 rank | bijou64 vs other best |
|-----------------|-----------|--------|-------|-------|--------|--------------|-----------------------|
| tiny (0-247)    | **1.18**  |  7.86  | 13.24 |  8.15 |  4.12  | #1           | 0.29x                 |
| small (248-64k) | **4.94**  |  9.53  | 15.04 | 10.39 | 11.12  | #1           | 0.52x                 |
| medium (64k-4B) | **5.88**  | 12.36  | 17.63 | 18.03 | 13.54  | #1           | 0.48x                 |
| large (>4B)     | **3.12**  | 20.33  | 12.28 | 16.24 | 38.23  | #1           | 0.25x                 |
| boundary        | **4.45**  | 15.66  | 14.67 | 12.13 | 11.34  | #1           | 0.39x                 |
| uniform random  | **3.30**  | 19.41  | 12.37 | 17.32 | 36.63  | #1           | 0.27x                 |

<details open>
<summary>Charts</summary>

![Stream Decode — Bar Chart](../charts/x86/stream_decode_bar.svg)
![Stream Decode — CDF](../charts/x86/stream_decode_cdf.svg)

</details>

bijou64 wins all 6 stream_decode cells, with margins from 1.93× (small vs varu64) up to 3.94× (large vs vu64). vu128 places 3rd on small/large/boundary/uniform, 4th on tiny (behind leb128 and varu64), and last on medium; the per-iteration `[u8; 9]` copy keeps it well behind bijou64 across the board.

<a id="vu128-copy-cost"></a>
> **vu128 copy cost.** `vu128::decode_u64` requires a `&[u8; 9]`. To decode from a `&[u8]` stream, each iteration copies up to 9 bytes from the cursor into a stack-allocated array. The cost is real and intrinsic to the `vu128` crate's API — a hypothetical alternative crate exposing a slice-input API would skip this — but the comparison here is between published crates as they actually ship. `bench_decode` and `bench_canonical_decode` use the identical pattern.

## Percentile Statistics

Full percentile breakdowns (p50/p90/p95/p99/p99.9) are available in:

- [`charts/x86/percentiles.md`](../charts/x86/percentiles.md) -- markdown tables
- [`charts/x86/percentiles.csv`](../charts/x86/percentiles.csv) -- machine-readable CSV
- [`charts/x86/percentiles.html`](../charts/x86/percentiles.html) -- interactive sortable table

Heatmaps provide a quick visual overview of which library performs best across all distributions:

<details>
<summary>Heatmaps (click to expand)</summary>

![Decode Heatmap](../charts/x86/decode_heatmap.svg)
![Canonical Decode Heatmap](../charts/x86/canonical_decode_heatmap.svg)
![Stream Decode Heatmap](../charts/x86/stream_decode_heatmap.svg)
![Encode Heatmap](../charts/x86/encode_heatmap.svg)

</details>

Interactive versions with hover-for-detail are in `charts/x86/*_heatmap.html`.

## Summary

On Zen 5, bijou64 wins **23 of 30** shootout cells outright:

| Operation        | Wins | Ties | Note                                              |
|------------------|-----:|-----:|---------------------------------------------------|
| encode           | 5/6  | 0    | Loses `small` to leb128                           |
| decode           | 6/6  | 0    |                                                   |
| encoded_size     | 0/6  | 0    | vu64 wins every cell; bijou64 is #2 in 3 of 6     |
| stream_decode    | 6/6  | 0    |                                                   |
| canonical_decode | 6/6  | 0    |                                                   |

bijou64 is the fastest decoder on every distribution, in every
benchmark variant (plain decode, canonical decode, stream decode),
by margins of roughly 1.9–4×. The narrowest margin (1.93×) is
`stream_decode/small` vs varu64; the widest (~4×) is
`stream_decode/large` vs vu64. Structural canonicality is free,
making bijou64 the unambiguous choice for protocols that decode
more than they encode and that need deterministic serialisation.

On the encode side, bijou64 wins 5 of 6 distributions; only `small`
goes to leb128 (1.24× faster on the 248–64k uniform range). leb128's
tight 2-byte-write loop happens to fit Zen 5's pipeline particularly
well for tier-2-heavy distributions.

The cells bijou64 loses are all format-bound:

- `encoded_size` non-tiny — ~2.2–2.5× behind vu64 across all 5
  distributions. vu64's power-of-2 boundaries let it skip the per-tier
  correction step bijou64 must perform; for `encoded_len`'s arithmetic-
  only path that delta is unavoidable.
- `encoded_size/tiny` — varu64 wins (~1.9× over bijou64).

These gaps are the price of bijective canonicality. For the
canonical-decode workloads that motivate bijou64, the trade is
overwhelmingly favourable.
