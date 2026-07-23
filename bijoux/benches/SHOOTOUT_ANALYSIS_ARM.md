# bijou64 Benchmark Shootout (ARM)

> Criterion benchmarks comparing bijou64 against varu64, vu64, vu128, and leb128 across six value distributions over batches of 4096 values.
>
> Run: `cargo bench -p bijoux --bench shootout`
>
> See also: [x86 results (AMD Ryzen AI 9 HX 370 / Zen 5)](SHOOTOUT_ANALYSIS_X86.md)

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
uv run bijoux/charts/analyze.py --arch arm
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

|         |                         |
|---------|-------------------------|
| CPU     | Apple M2 Pro            |
| Memory  | 32 GB                   |
| OS      | macOS 26.3.1            |
| Rust    | 1.94.0                  |
| Profile | `bench` (opt-level = 3) |

> All medians are taken from [`charts/arm/percentiles.md`](../charts/arm/percentiles.md).

## Encode

Encode to a `Vec<u8>`.

| Distribution    | bijou64      | varu64 | vu64         | vu128 | leb128       | bijou64 rank | bijou64 vs other best |
|-----------------|--------------|--------|--------------|-------|--------------|--------------|----------------------|
| tiny (0-247)    | **2.16** | 10.89  | 20.72        | 16.55 | 3.62         | #1           | 0.60x                |
| small (248-64k) | 8.54         | 20.53  | 22.42        | 19.59 | **7.46** | #2           | 1.14x                |
| medium (64k-4B) | **8.94** | 25.61  | 21.75        | 21.08 | 12.13        | #1           | 0.74x                |
| large (>4B)     | **9.25** | 27.19  | 10.48        | 11.30 | 30.48        | #1           | 0.88x                |
| boundary        | **8.03** | 28.63  | 17.85        | 17.83 | 10.76        | #1           | 0.75x                |
| uniform random  | **9.07** | 29.04  | 10.94        | 11.95 | 32.50        | #1           | 0.83x                |

<details open>
<summary>Charts</summary>

![Encode — Bar Chart](../charts/arm/encode_bar.svg)
![Encode — Box Plot](../charts/arm/encode_box.svg)
![Encode — CDF](../charts/arm/encode_cdf.svg)

</details>

bijou64 wins 5 of 6 encode distributions on M2 Pro. The sole loss is `small` to leb128 (1.14x behind), where leb128's tight 2-byte write loop fits well. The encode improvements from the shift+truncate trick are even more pronounced on ARM than on Zen 5 -- medium and boundary distributions that previously lost are now clear wins.

## Encoded Size Query (runtime)

Wall-clock time to call `encoded_len(v)` across the 6 distributions.
vu128 and leb128 are excluded: neither crate exposes a standalone
`encoded_len(u64)` query (both compute size only as a side effect of
encoding).

> For the arch-independent _format_ size comparison (how many bytes
> each format uses), see [SIZE_ANALYSIS.md](../design/SIZE_ANALYSIS.md).

| Distribution    | bijou64  | varu64   | vu64     | bijou64 rank        | bijou64 vs other best |
|-----------------|---------:|---------:|---------:|---------------------|-----------------------|
| tiny (0-247)    | 1.37     | 1.37     | **1.00** | tied #2 with varu64 | 1.37x                 |
| small (248-64k) | 2.94     | 2.66     | **1.00** | #3                  | 2.95x                 |
| medium (64k-4B) | 2.93     | 5.39     | **1.00** | #2                  | 2.93x                 |
| large (>4B)     | 2.99     | 6.62     | **1.00** | #2                  | 3.01x                 |
| boundary        | 2.94     | 6.04     | **1.01** | #2                  | 2.92x                 |
| uniform random  | 2.90     | 6.05     | **0.92** | #2                  | 3.17x                 |

<details open>
<summary>Charts</summary>

![Encoded Size — Bar Chart](../charts/arm/encoded_size_bar.svg)
![Encoded Size — Box Plot](../charts/arm/encoded_size_box.svg)
![Encoded Size — CDF](../charts/arm/encoded_size_cdf.svg)

</details>

vu64 wins every cell because its tier boundaries are exact powers of
2, so `encoded_len` reduces to a single `clz` with no correction step.
bijou64's per-tier offsets force an extra comparison; the gap
(~2.9-3.2x) is unavoidable for the canonicality-preserving path. On
`tiny`, bijou64 and varu64 are a statistical tie (1.369 us vs 1.372
us; the difference is well inside criterion's confidence interval).
See [OPTIMISATION.md](../design/OPTIMISATION.md) for the full analysis.

## Decode

Decode from a `&[u8]` buffer.

| Distribution    | bijou64      | varu64 | vu64  | vu128 | leb128 | bijou64 rank | bijou64 vs other best |
|-----------------|--------------|--------|-------|-------|--------|--------------|----------------------|
| tiny (0-247)    | **2.62** | 5.27   | 15.66 | 22.42 | 6.65   | #1           | 0.50x                |
| small (248-64k) | **6.09** | 10.53  | 13.21 | 14.71 | 14.36  | #1           | 0.58x                |
| medium (64k-4B) | **6.09** | 17.24  | 15.26 | 11.30 | 17.23  | #1           | 0.54x                |
| large (>4B)     | **6.78** | 23.76  | 9.42  | 9.28  | 38.86  | #1           | 0.73x                |
| boundary        | **7.40** | 20.19  | 12.08 | 10.88 | 15.43  | #1           | 0.68x                |
| uniform random  | **6.69** | 23.82  | 9.34  | 9.27  | 37.81  | #1           | 0.72x                |

<details open>
<summary>Charts</summary>

![Decode — Bar Chart](../charts/arm/decode_bar.svg)
![Decode — Box Plot](../charts/arm/decode_box.svg)
![Decode — CDF](../charts/arm/decode_cdf.svg)

</details>

bijou64 wins every decode cell on M2 Pro, with margins ranging from 1.4x (large vs vu128) to 5.7x (uniform vs leb128). The `#[inline]` on `decode` is the key enabler -- previously, bijou64 lost large, boundary, and uniform to vu64/vu128 on this machine.

## Canonical Decode

Decode with a guarantee that the encoding is minimal (no overlong representations accepted). This matters for protocols that need deterministic serialisation -- if two peers can encode the same value differently, content-addressed hashes break.

bijou64 achieves canonicality structurally: its disjoint tier ranges make overlong encodings impossible, so the canonical decode path is identical to regular decode with zero overhead. varu64 and vu64 always perform a runtime minimality check (there's no way to opt out). vu128 and leb128 accept overlong encodings by design, so we wrap them with a decode-then-re-encode-and-compare-length check to simulate what a canonical-aware caller would need to do.

| Distribution    | bijou64      | varu64 | vu64  | vu128 | leb128 | bijou64 rank | bijou64 vs other best |
|-----------------|--------------|--------|-------|-------|--------|--------------|----------------------|
| tiny (0-247)    | **2.46** | 4.96   | 17.97 | 21.69 | 15.95  | #1           | 0.50x                |
| small (248-64k) | **5.69** | 9.90   | 12.92 | 19.90 | 23.30  | #1           | 0.57x                |
| medium (64k-4B) | **5.69** | 16.18  | 14.42 | 14.56 | 28.08  | #1           | 0.39x                |
| large (>4B)     | **6.32** | 22.24  | 8.79  | 13.59 | 55.17  | #1           | 0.72x                |
| boundary        | **7.46** | 19.06  | 11.24 | 15.74 | 26.82  | #1           | 0.66x                |
| uniform random  | **6.27** | 22.23  | 8.75  | 13.59 | 54.60  | #1           | 0.72x                |

<details open>
<summary>Charts</summary>

![Canonical Decode — Bar Chart](../charts/arm/canonical_decode_bar.svg)
![Canonical Decode — CDF](../charts/arm/canonical_decode_cdf.svg)

</details>

The cost of canonicality varies wildly by crate. bijou64's numbers are identical to plain decode because there's nothing extra to check. varu64 always pays its runtime check, so its column matches the plain decode table. vu128 and leb128 pay the round-trip re-encode penalty -- leb128 in particular is catastrophic on large/uniform (55 us vs 39 us without the check) because of its byte-at-a-time `Write`/`Read` interface.

bijou64 wins all 6 canonical decode distributions on M2 Pro. For protocols that _require_ canonical encoding, this is the table that matters.

## Stream Decode

Decode a concatenated stream of encoded values, advancing a cursor by the number of bytes consumed per call.

> [!NOTE]
> The table below predates the inclusion of vu128 in the stream_decode bench (added 2026-05-27; vu128 uses the same per-iteration `[u8; 9]` copy as plain decode — see [SHOOTOUT_ANALYSIS_X86.md](SHOOTOUT_ANALYSIS_X86.md#vu128-copy-cost)). The four-column numbers below remain valid for the listed codecs, but a vu128 column and any potential score adjustment require re-running the shootout on ARM hardware. The "wins 6/6" claim below is provisional on that re-bench. On x86 the addition did not flip any stream_decode cell.

| Distribution    | bijou64      | varu64 | vu64  | leb128 | bijou64 rank | bijou64 vs other best |
|-----------------|--------------|--------|-------|--------|--------------|----------------------|
| tiny (0-247)    | **1.82** | 10.97  | 16.58 | 5.85   | #1           | 0.31x                |
| small (248-64k) | **4.95** | 30.29  | 16.67 | 12.59  | #1           | 0.39x                |
| medium (64k-4B) | **4.82** | 16.09  | 14.19 | 15.59  | #1           | 0.34x                |
| large (>4B)     | **5.65** | 24.35  | 8.76  | 34.87  | #1           | 0.64x                |
| boundary        | **6.26** | 22.11  | 14.13 | 14.34  | #1           | 0.44x                |
| uniform random  | **5.58** | 23.85  | 10.70 | 34.47  | #1           | 0.52x                |

<details open>
<summary>Charts</summary>

![Stream Decode — Bar Chart](../charts/arm/stream_decode_bar.svg)
![Stream Decode — CDF](../charts/arm/stream_decode_cdf.svg)

</details>

bijou64 wins every stream_decode cell against varu64/vu64/leb128. The tiny distribution is particularly striking: 1.82 us vs 5.85 us for the next-best (leb128), a 3.2x margin. vu128 ranking pending ARM re-bench.

## Percentile Statistics

Full percentile breakdowns (p50/p90/p95/p99/p99.9) are available in:

- [`charts/arm/percentiles.md`](../charts/arm/percentiles.md) -- markdown tables
- [`charts/arm/percentiles.csv`](../charts/arm/percentiles.csv) -- machine-readable CSV
- [`charts/arm/percentiles.html`](../charts/arm/percentiles.html) -- interactive sortable table

Heatmaps provide a quick visual overview of which library performs best across all distributions:

<details>
<summary>Heatmaps (click to expand)</summary>

![Decode Heatmap](../charts/arm/decode_heatmap.svg)
![Canonical Decode Heatmap](../charts/arm/canonical_decode_heatmap.svg)
![Stream Decode Heatmap](../charts/arm/stream_decode_heatmap.svg)
![Encode Heatmap](../charts/arm/encode_heatmap.svg)

</details>

Interactive versions with hover-for-detail are in `charts/arm/*_heatmap.html`.

## Summary

On M2 Pro, bijou64 wins **23 of 30** shootout cells outright, plus 1
shared second place:

| Operation        | Wins | Ties | Note                                                              |
|------------------|-----:|-----:|-------------------------------------------------------------------|
| encode           | 5/6  | 0    | Loses `small` to leb128                                           |
| decode           | 6/6  | 0    |                                                                   |
| encoded_size     | 0/6  | 1    | vu64 wins every cell; bijou64 ties varu64 for #2 on `tiny`        |
| stream_decode    | 6/6* | 0    | *vs varu64/vu64/leb128; vu128 ranking pending ARM re-bench        |
| canonical_decode | 6/6  | 0    |                                                                   |

This matches the Zen 5 outright-win score exactly (23/30), confirming that the optimisations transfer cleanly across microarchitectures.

bijou64 is the fastest decoder on every distribution, in every benchmark variant (plain decode, canonical decode, stream decode), typically by 1.4--3.2x margins. Structural canonicality is free, making bijou64 the unambiguous choice for protocols that decode more than they encode and that need deterministic serialisation.

On the encode side, bijou64 wins 5 of 6 distributions; only `small` goes to leb128 (1.14x faster on the 248--64k uniform range). leb128's tight 2-byte-write loop fits both M2 and Zen 5 well for tier-2-heavy distributions.

The cells bijou64 loses are all format-bound:

- `encoded_size` non-tiny -- ~2.9x behind vu64 across all 5 distributions. vu64's `encoded_len` reduces to a single `clz` with no correction step; bijou64's per-tier offsets force a correction.
- `encoded_size/tiny` -- statistical tie with varu64 (both ~1.37 us).

These gaps are the price of bijective canonicality. For the canonical-decode workloads that motivate bijou64, the trade is overwhelmingly favourable.
