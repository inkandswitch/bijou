# bijou128 Benchmark Shootout (ARM)

> [!NOTE]
> **Not yet run on ARM.** This document is a placeholder.
>
> To populate: run the shootout on a current Apple Silicon machine and
> regenerate the charts via `uv run bijou128/charts/analyze.py --arch arm`.
> Until that happens, treat this file as a stub.

> Criterion benchmarks comparing bijou128 against `vu128` across seven
> value distributions over batches of 4096 values.
>
> Run: `cargo bench -p bijou128 --bench shootout`
>
> See also: [x86 results (AMD Ryzen AI 9 HX 370 / Zen 5)](SHOOTOUT_ANALYSIS_X86.md) — _current_

## To populate

```bash
# On an ARM (Apple Silicon) machine:
cargo bench -p bijou128 --bench shootout
uv run bijou128/charts/analyze.py --arch arm
```

This will produce `bijou128/charts/arm/*.svg`/`.html` and a fresh
`percentiles.md`. Then mirror this file's structure from
[`SHOOTOUT_ANALYSIS_X86.md`](SHOOTOUT_ANALYSIS_X86.md) with the new
numbers, replacing the placeholder Machine table.

## Expected differences from x86

Based on bijou64's ARM results (Apple M2 Pro), here's what to anticipate
for bijou128:

- **Encode**: vu128 advantage may persist on ARM. The bijou family's
  per-tier offset correction is format-bound; M2's wider pipeline didn't
  help bijou64 close the analogous `encode_array` vs vu64 gap.
- **Decode**: bijou128 should still sweep — the `#[inline]` decode trick
  works on both architectures, and the structural canonicality advantage
  is portable.
- **Canonical decode**: bijou128's structural-canonicality win is
  microarchitecture-agnostic; vu128's round-trip overhead transfers
  directly.
- **Stream decode / encoded size**: bijou128-only by API limitation, not
  by performance.

## Machine (when populated)

|         |                         |
|---------|-------------------------|
| CPU     | _Apple M2 Pro (TBD)_    |
| Memory  | _TBD_                   |
| OS      | _macOS (TBD)_           |
| Rust    | _1.90+_                 |
| Profile | `bench` (opt-level = 3) |
