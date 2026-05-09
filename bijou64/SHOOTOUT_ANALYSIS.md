# bijou64 Benchmark Shootout

> Criterion benchmarks comparing bijou64 against varu64, vu64, vu128, and leb128 across six value distributions over batches of 4096 values.

## Running the Shootout

### 1. Run the Benchmarks

```bash
cargo bench -p bijou64 --bench shootout
```

This writes Criterion sample data to `target/criterion/`.

### 2. Generate Charts

The `--arch` flag controls the output subdirectory so that each architecture keeps its own charts. If omitted, it auto-detects from `uname -m`.

```bash
# via nix flake app
nix run .#bench-charts -- --arch x86    # writes to bijou64/charts/x86/
nix run .#bench-charts -- --arch arm    # writes to bijou64/charts/arm/

# or via uv (auto-installs Python deps)
uv run bijou64/charts/analyze.py --arch x86
uv run bijou64/charts/analyze.py --arch arm

# auto-detect architecture (x86_64 → x86, aarch64 → arm)
uv run bijou64/charts/analyze.py
```

Output lands in `bijou64/charts/<arch>/`:

```
bijou64/charts/<arch>/
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

| Architecture | CPU                            | Results                                            |
|--------------|--------------------------------|----------------------------------------------------|
| x86_64       | AMD Ryzen AI 9 HX 370 (Zen 5)  | [SHOOTOUT_ANALYSIS_X86.md](SHOOTOUT_ANALYSIS_X86.md) |
| AArch64      | Apple M2 Pro                   | [SHOOTOUT_ANALYSIS_ARM.md](SHOOTOUT_ANALYSIS_ARM.md) — ⚠️ stale |

## Quick Comparison

bijou64 cell-by-cell standing per architecture:

| Benchmark        | x86 (Zen 5)                              | ARM (M2 Pro) ⚠️ stale       |
|------------------|------------------------------------------|------------------------------|
| Encode (Vec)     | Wins 5/6 (loses `small`)                 | Wins 1/6 (`tiny`)            |
| Decode           | Wins 6/6                                 | Wins 3/6 (`tiny`, `small`, `medium`) |
| Canonical Decode | Wins 6/6                                 | Wins 4/6                     |
| Stream Decode    | Wins 6/6                                 | Wins 3/6 (`tiny`, `small`, `medium`) |
| Encoded Size     | tied `tiny`; 2nd–3rd elsewhere           | 2nd–3rd                      |
| **Total**        | **23/30**                                | **14/30**                    |

> The ARM column is from an older bench run on an earlier
> version of the code; numbers will shift when ARM is re-run.

On Zen 5, bijou64 is the fastest decoder on every distribution
across all three decode variants (plain, canonical, stream), and the
fastest encoder on 5 of 6 distributions. The cells it loses
(`encoded_size` non-tiny, `encode/small`) are either format-bound
(vu64's power-of-2 boundaries skip a correction step bijou64 must
perform) or pipeline-bound on a specific size range.

For workloads that care about canonical encoding — content-addressed
hashes, deterministic serialisation — bijou64's structural
canonicality means the canonical decode path is identical to plain
decode, while non-canonical formats pay 2–3× more (leb128 hits 64 µs
on `large/uniform` canonical decode vs 33 µs without the check).
