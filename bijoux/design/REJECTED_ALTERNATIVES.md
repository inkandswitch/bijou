# Rejected Signed-Format Alternatives

Two candidate designs for the signed bijou formats were prototyped,
benchmarked, and rejected during design (2026-07). This file is the
evidence backing the rejection summaries in the
[bijou64s spec](../../specs/bijou64s.md)'s Prior Art section; the
shipped format's numbers against *external* codecs live in
[SHOOTOUT_ANALYSIS_SIGNED.md](SHOOTOUT_ANALYSIS_SIGNED.md).

Both rejected lanes remain implemented in
[`signed_shootout.rs`](signed_shootout.rs) (`bijou64+twos` and
`mod mirrored`) so these results stay reproducible. Setup as in the
main analysis: Zen 5, criterion medians, µs per 4096-value batch,
bold = lowest in row. Rows `tiny…uniform` from the 2026-07-22 run;
the signed-realistic rows (`deltas…hexane`) from the 2026-07-22b run.

## The candidates

### Two's-complement passthrough (`bijou+twos`)

`encode_u64(v as u64)` — zero mapping cost, but two's complement puts
every negative value's bit pattern at the top of the unsigned range,
so **all negatives encode as the maximum 9 bytes** regardless of
magnitude.

### Sign-in-tag / mirrored tiers (`mirrored`)

A bespoke first-byte layout — negative length tags `0x00..=0x07`,
direct window `[-120, +119]`, positive tags `0xF8..=0xFF`, negative
payloads complemented — giving structural canonicality **and**
byte-lexicographic = numeric order (the one property zigzag gives up).
The prototype is verified (round-trip, consumed-length, and full
lex-order checks run before any timing) and includes the same
optimisations as the shipped format (truncate-trick encode, `#[inline]`
decode, top-tier-only overflow checks).

## Decode (the decisive table)

| Distribution | bijou64s (shipped) | bijou+twos | mirrored |
|--------------|-------------------:|-----------:|---------:|
| tiny ±124    |           **2.00** |       2.24 |     5.68 |
| small ±32k   |               4.80 |   **3.33** |     16.6 |
| medium ±2³¹  |               4.76 |   **3.35** |     19.7 |
| medium all+  |               4.71 |   **4.04** |     8.79 |
| medium all−  |               4.75 |   **3.06** |     13.4 |
| large        |               3.95 |   **3.06** |     29.6 |
| boundary     |               6.55 |   **3.46** |     14.7 |
| uniform      |           **4.15** |       4.55 |     26.4 |
| deltas ±16   |           **2.31** |       2.46 |     4.13 |
| deltas geom  |               2.47 |   **2.45** |     4.39 |
| hexane mix   |           **3.18** |       3.63 |     9.57 |

## Density (bytes/value — why twos' speed doesn't matter)

| Distribution |  bijou64s | bijou+twos |  mirrored |
|--------------|----------:|-----------:|----------:|
| tiny ±124    | **1.000** |      5.000 |     1.030 |
| medium ±2³¹  | **4.997** |      7.033 |     4.992 |
| medium all−  | **4.995** |      9.000 |     4.990 |
| deltas ±16   | **1.000** |      4.863 | **1.000** |
| deltas geom  | **1.006** |      4.848 |     1.007 |
| hexane mix   |     1.510 |      5.209 | **1.490** |

## Verdicts

### Two's complement: rejected on density

Its raw-speed wins are an artifact of the density failure: every
negative takes one fixed-size, perfectly-predicted 9-byte path. On the
most common signed values the cost is catastrophic (`-1` = 9 bytes;
~5 B/val on delta-shaped data vs the shipped format's ~1.0–1.5), and
byte order is broken (negatives sort above positives).

### Mirrored tiers: rejected on decode speed

Encode reaches parity with the shipped format once the same
optimisations are applied, and density is a wash (it even edges the
shipped format by ~1 % on the hexane mix). But decode is structurally
2–3× slower on sign-homogeneous data and up to ~6× on mixed signs.
Investigation (three hypotheses, tested):

- **Sign entropy becomes branch entropy — confirmed, ~half the
  penalty.** Zigzag folds the sign into the value *before* tag
  dispatch, so mixed-sign streams still hit one hot arm per tier;
  mirrored's per-sign tags turn sign randomness into dispatch
  randomness (mixed 19.7 µs vs all-positive 8.79 / all-negative 13.4).
- **Checked-overflow plumbing — falsified** (restricting checks to the
  top tier changed nothing).
- **Dispatch structure (17 arms vs 9) — unattributed residual**;
  would need instruction-level tooling to pin down.

Decode is the hot path in the target workloads, so lex ordering — the
lowest-ranked design priority — was not worth 2–6×. Measured on Zen 5
only; the structural branch-target argument is µarch-independent. A
memcomparable signed format is explicitly out of scope; if that ever
changes, the verified prototype in the bench is the starting point.
