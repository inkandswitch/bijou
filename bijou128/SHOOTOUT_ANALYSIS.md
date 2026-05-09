# bijou128 Benchmark Shootout

> Criterion benchmarks comparing bijou128 against `vu128` across seven
> value distributions over batches of 4096 values.
>
> `vu128` is the only u128 varint library on crates.io. The other
> bijou64 shootout competitors (`varu64`, `vu64`, `leb128`) are u64-only
> and don't apply here.

## Running the Shootout

### 1. Run the Benchmarks

```bash
cargo bench -p bijou128 --bench shootout
```

This writes Criterion sample data to `target/criterion/`.

### 2. Generate Charts

```bash
# via uv (auto-installs Python deps)
uv run bijou128/charts/analyze.py --arch x86
uv run bijou128/charts/analyze.py --arch arm

# auto-detect architecture (x86_64 → x86, aarch64 → arm)
uv run bijou128/charts/analyze.py
```

Output lands in `bijou128/charts/<arch>/`:

```
bijou128/charts/<arch>/
  percentiles.csv       # machine-readable statistics
  percentiles.md        # markdown tables (p50/p90/p95/p99/p99.9)
  percentiles.html      # interactive sortable table
  *_bar.svg             # grouped bar charts (median + min–p95 whiskers)
  *_box.svg             # box-and-whisker plots
  *_cdf.svg             # CDF overlay plots
  *_heatmap.svg         # library × distribution heatmaps
  *_cdf.html            # interactive Plotly CDFs
  *_heatmap.html        # interactive Plotly heatmaps
```

## Results by Architecture

| Architecture | CPU                            | Results                                              |
|--------------|--------------------------------|------------------------------------------------------|
| x86_64       | AMD Ryzen AI 9 HX 370 (Zen 5)  | [SHOOTOUT_ANALYSIS_X86.md](SHOOTOUT_ANALYSIS_X86.md) |
| AArch64      | Apple M2 Pro                   | [SHOOTOUT_ANALYSIS_ARM.md](SHOOTOUT_ANALYSIS_ARM.md) — ⏳ not yet run |

## Quick Comparison (Zen 5 only)

bijou128 cell-by-cell standing vs `vu128`:

| Benchmark        | x86 (Zen 5)                       |
|------------------|-----------------------------------|
| Encode (Vec)     | Wins 1/7 (`tiny`)                 |
| Decode           | Wins 7/7                          |
| Canonical Decode | Wins 7/7                          |
| Stream Decode    | bijou128 only (vu128 lacks API)   |
| Encoded Size     | bijou128 only (vu128 lacks API)   |

### Cell-by-cell summary against `vu128`

bijou128 sweeps every decode-side cell (plain, canonical, stream) by
significant margins (0.39×–0.82× of vu128's time). Canonical decode
is particularly wide — vu128 needs to round-trip-and-compare to enforce
canonicality, while bijou128's structural canonicality means the cost
is _zero_ over plain decode.

On the encode side, `vu128`'s prefix-bit format is faster than
bijou128's tag+offset scheme for multi-byte tiers (1.29×–1.66× faster).
The pattern is the same shape as bijou64 had vs `vu64` on `encode_array`
before the bijou64 perf work: vu128 doesn't pay for the per-tier
correction bijou128 must do. Tiny-value encode goes to bijou128 by 0.69×.

## Distribution choice

Seven distributions cover progressively wider ranges, including two
specifically for `u128` territory:

| Distribution         | Range                          | Tier   |
|----------------------|--------------------------------|--------|
| tiny (0-239)         | single-byte tier 0             | 0      |
| small (240-65k)      | 240..65 535                    | 1–2    |
| medium (64k-4G)      | 65 536..u32::MAX               | 2–4    |
| large (u64)          | u32::MAX+1..u64::MAX           | 5–8    |
| xlarge (u128)        | u64::MAX+1..u128::MAX          | 9–16   |
| boundary             | every tier-edge value, cycled  | mixed  |
| uniform_random       | full u128 range                | mixed  |

The split between `large (u64)` and `xlarge (u128)` is bijou128-specific:
`xlarge` exercises the multi-byte tiers above 8 that don't exist in
bijou64.

## When to use bijou128

- If your protocol decodes more than it encodes (most do — encode happens
  once per write, decode per read), bijou128 is uniformly faster than
  vu128.
- If your protocol requires canonical encoding (content-addressed hashes,
  deterministic serialisation), bijou128's structural canonicality means
  the canonical decode path is _identical_ to plain decode, while vu128
  pays a 50–95% canonicality tax.
- If your hot path is encoding a large number of values, vu128 is faster
  on the multi-byte path; consider bijou128 only if the canonical
  property matters.
