# Signed Shootout Analysis

_Captured 2026-07-22 from a single complete run of the shipped
`bijoux::i64` module (criterion lane id `bijou64s`) against the
published signed varint crates._

## Setup

|         |                                                                                                   |
|---------|---------------------------------------------------------------------------------------------------|
| Bench   | `bijoux/benches/signed_shootout.rs`                                                               |
| Command | `cargo bench -p bijoux --bench signed_shootout -- --warm-up-time 1 --measurement-time 2 --noplot` |
| Machine | AMD Ryzen AI 9 HX 370 (Zen 5), NixOS — same machine as the other analyses in this directory       |
| Profile | `bench` (`opt-level = 3`, `lto = "thin"`, `codegen-units = 1`)                                    |
| Batch   | 4096 values per distribution, seed `0xBEEF_CAFE_DEAD_F00D`                                        |

Speed values are criterion medians in **µs per 4096-value batch**;
lower is better. **Bold = lowest in row.** Expect ~5–15 % run-to-run
variance; treat sub-10 % gaps as ties. The `boundary` distribution
(adversarial tier-edge alternation) is the suite's noisiest cell.

## Lanes

| Lane       | What it is                                                                             |
|------------|----------------------------------------------------------------------------------------|
| `bijou64s` | the shipped `bijoux::i64` module (wire format bijou64s: zigzag over the bijou64 tiers) |
| `vu64+zz`  | zigzag, then the `vu64` crate                                                          |
| `vu128`    | `vu128` crate's native signed API (zigzag internally)                                  |
| `sleb128`  | `leb128` crate's native signed LEB128                                                  |

Distributions: `medium ±2³¹` draws magnitudes 32 768..2³¹ with random
sign; `medium all+` / `medium all−` fix the sign.

The first eight distributions mirror the unsigned shootout's bands
around zero — useful for coverage, but they overweight regions real
signed data rarely occupies. The last three model how signed integers
actually occur:

- `deltas ±16` — uniform micro-deltas (cursor/clock steps)
- `deltas geom` — two-sided geometric magnitudes (p = 0.96, mean
  |v| ≈ 24): a generic decaying-delta model
- `hexane mix` — shaped by the hexane census (1.35M real delta values
  from the automerge 260k-edit trace): 74 % in the 1-byte window,
  7.3 % in the rest of |v| ≤ 256, 18.8 % in the measured bit-length
  tail

## Encode

| Distribution | bijou64s | vu64+zz | vu128 |  sleb128 |
|--------------|---------:|--------:|------:|---------:|
| tiny ±124    | **2.37** |    20.6 |  10.7 |     4.35 |
| small ±32k   |     10.8 |    21.5 |  15.0 | **8.17** |
| medium ±2³¹  | **11.4** |    21.2 |  15.1 |     14.4 |
| medium all+  | **11.2** |    22.0 |  21.6 |     22.4 |
| medium all−  | **11.9** |    21.5 |  15.5 |     14.3 |
| large        | **12.3** |    31.3 |  19.2 |     32.3 |
| boundary     | **12.4** |    26.6 |  18.0 |     14.2 |
| uniform      | **15.0** |    27.3 |  17.2 |     28.1 |
| deltas ±16   | **2.74** |    25.1 |  7.54 |     3.71 |
| deltas geom  | **3.72** |    24.5 |  10.8 |     3.78 |
| hexane mix   | **5.48** |    24.9 |  13.0 |     5.94 |

## Decode

| Distribution |   bijou64s | vu64+zz | vu128 | sleb128 |
|--------------|-----------:|--------:|------:|--------:|
| tiny ±124    |   **2.00** |    9.74 |  7.94 |    4.55 |
| small ±32k   |   **4.80** |    13.1 |  10.3 |    6.12 |
| medium ±2³¹  |   **4.76** |    15.8 |  10.7 |    10.3 |
| medium all+  |   **4.71** |    15.9 |  9.80 |    10.2 |
| medium all−  |   **4.75** |    15.4 |  9.64 |    10.2 |
| large        |   **3.95** |    12.3 |  10.0 |    24.9 |
| boundary     | **6.55**\* |    15.9 |  9.28 |    9.95 |
| uniform      |   **4.15** |    9.05 |  7.87 |    18.0 |
| deltas ±16   |   **2.31** |    8.26 |  7.78 |    4.35 |
| deltas geom  |   **2.47** |    11.5 |  10.7 |    5.31 |
| hexane mix   |   **3.18** |    15.5 |  16.4 |    11.0 |

\* Transient outlier: a longer re-measure immediately after gave
5.64 µs. The boundary distribution is adversarial for branch
predictors.

## Stream decode

| Distribution | bijou64s | vu64+zz | vu128 | sleb128 |
|--------------|---------:|--------:|------:|--------:|
| tiny ±124    | **1.64** |    10.3 |  7.63 |    4.08 |
| small ±32k   | **4.68** |    13.1 |  8.73 |    5.91 |
| medium ±2³¹  | **4.58** |    14.8 |  14.6 |    11.6 |
| medium all+  | **4.11** |    14.8 |  14.8 |    11.3 |
| medium all−  | **4.41** |    14.1 |  14.3 |    10.8 |
| large        | **3.16** |    9.97 |  13.2 |    22.5 |
| boundary     | **3.93** |    13.9 |  10.6 |    10.7 |
| uniform      | **3.23** |    9.90 |  13.3 |    22.5 |
| deltas ±16   | **2.07** |    11.2 |  6.46 |    3.98 |
| deltas geom  | **2.20** |    13.5 |  9.36 |    4.78 |
| hexane mix   | **2.62** |    12.2 |  7.90 |    5.01 |

## Encoded-length query (`encoded_len` speed, not size)

| Distribution | bijou64s |  vu64+zz |
|--------------|---------:|---------:|
| tiny ±124    |     1.82 | **1.72** |
| small ±32k   |     3.01 | **1.82** |
| medium ±2³¹  |     3.07 | **1.77** |
| large        |     3.12 | **1.78** |
| boundary     |     2.92 | **1.79** |
| uniform      |     3.15 | **1.70** |

vu64's power-of-two tiers win the length query, as in the unsigned
shootout — format-bound; see `../docs/OPTIMISATION.md`.

## Density (bytes/value)

Deterministic (format property, not a timing).

| Distribution |  bijou64s |   vu64+zz |     vu128 |   sleb128 |
|--------------|----------:|----------:|----------:|----------:|
| tiny ±124    | **1.000** |     1.486 |     1.486 |     1.486 |
| small ±32k   |     2.995 | **2.731** | **2.731** | **2.731** |
| medium ±2³¹  |     4.997 | **4.934** | **4.934** | **4.934** |
| medium all+  |     4.997 | **4.937** | **4.937** | **4.937** |
| medium all−  |     4.995 | **4.939** | **4.939** | **4.939** |
| large        | **8.996** | **8.996** | **8.996** |     9.497 |
| boundary     |     4.995 | **4.773** |     5.106 |     4.828 |
| uniform      | **8.996** | **8.996** | **8.996** |     9.496 |
| deltas ±16   | **1.000** | **1.000** | **1.000** | **1.000** |
| deltas geom  | **1.006** |     1.073 |     1.073 |     1.073 |
| hexane mix   | **1.510** |     1.718 |     1.718 |     1.718 |

## Reading the results

- `bijou64s` beats every external lane on every decode and
  stream-decode cell, and every encode cell except small-vs-sleb128.
  Decode tiny at 2.0 µs per 4096 values ≈ **2 billion values/sec**
  (~0.5 ns/value on single-byte encodings).
- Density is within ~1 % of the 7-bit-per-byte family on multi-byte
  tiers and 49 % better on the single-byte window (±124 vs ±63).
- **On the signed-realistic distributions the density question
  resolves in bijou64s's favour**: the synthetic `small ±32k` band
  (where bijou's byte-granular tiers cost ~10 % vs LEB's 7-bit tiers)
  is a transit zone real data passes through, not parks in. On the
  census-shaped `hexane mix`, bijou64s is **12 % denser** than SLEB128
  (1.510 vs 1.718 B/val) *and* 3.5× faster to decode; on geometric
  deltas, 6 % denser. The fat 1-byte window wins more than the
  mid-band loses — which is why the tier geometry is not fork-worthy
  for the signed formats.
- Alternative in-house signed designs (a two's-complement passthrough
  and a sign-in-tag layout) were prototyped, benchmarked, and rejected
  during design; see [REJECTED_ALTERNATIVES.md](../design/REJECTED_ALTERNATIVES.md).
