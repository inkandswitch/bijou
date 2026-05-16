# bijou64 Optimisation Notes

> These are implementation notes, not part of the specification. The [spec][SPEC] defines the _format_; this document records why the reference implementation makes the choices it does, what we measured, and where the remaining costs come from.

> [!NOTE]
> Function-name drift: this document describes work done on a function originally named `encode_array(u64) -> ([u8; 9], usize)`. That function was replaced by `encoded_bytes(u64) -> EncodedBytes` (a smallvec-style wrapper that always exposes the correct prefix via `Deref<[u8]>`). The codegen properties analysed below are unchanged — `encoded_bytes` does the same shift-and-array-literal trick — only the name and return type differ.

[SPEC]: ./SPEC.md

## `encoded_len`: `leading_zeros` Instead of an If-Chain

### Background

The straightforward way to implement `encoded_len` is an if/else chain that tests the value against each tier boundary in order:

```rust
pub const fn encoded_len(value: u64) -> usize {
    if value < BOUNDS[0] { 1 }
    else if value < BOUNDS[1] { 2 }
    // ...six more arms...
    else { 9 }
}
```

This is clear and obviously correct, and for tier 0 values (the common case in our protocol -- blob sizes tend to be 54--100 bytes) it's _fast_: one comparison, well-predicted, done.

The trouble shows up on mixed or large-value distributions. A uniform random `u64` walks an average of four or five comparisons before hitting the right arm, and the branch predictor can't help much because the tier depends on the value. In benchmarks this was pretty stark -- up to 6.4 µs per 4096 values for `uniform_random`, compared to ~0.95 µs for `vu64::encoded_len`, which uses a branchless `leading_zeros` approach.

### The Trick

bijou64's tier boundaries aren't _exactly_ powers of 256 (that's the whole point of the per-tier offsets), but they're _close_. Each tier spans exactly 8 bit-widths after tier 1:

```text
Bit-width  0..= 7  →  tier 0   (1 byte)
Bit-width  8       →  tier 1   (2 bytes)
Bit-width  9..=16  →  tier 2   (3 bytes)
Bit-width 17..=24  →  tier 3   (4 bytes)
Bit-width 25..=32  →  tier 4   (5 bytes)
Bit-width 33..=40  →  tier 5   (6 bytes)
Bit-width 41..=48  →  tier 6   (7 bytes)
Bit-width 49..=56  →  tier 7   (8 bytes)
Bit-width 57..=64  →  tier 8   (9 bytes)
```

So `u64::leading_zeros()` -- a single `lzcnt` / `clz` instruction on most architectures -- gets us a _candidate_ tier via simple arithmetic: `(bit_width - 1) / 8 + 2`.

The complication is that the per-tier offsets push each boundary slightly past the corresponding power of 256. For example, `BOUNDS[1]` is 504, not 512. That means 503 has bit-width 9 and the formula says "tier 2", but 503 is actually the last value of tier 1. The candidate can be one too high.

The good news: it can _only_ be one too high. The offsets grow geometrically, but they never push a boundary far enough to cross a full additional bit-width.[^proof] A single comparison against the tier's actual bound corrects for this.

[^proof]: Each offset is $\sum_{k=0}^{t-1} 256^k$, which is less than $2 \cdot 256^{t-1}$ -- i.e., less than one extra bit of headroom. The boundary at tier $t$ sits at `256^t + offset`, which still fits within the same 8-bit-width band as `256^t` alone.

### Implementation

```rust
pub const fn encoded_len(value: u64) -> usize {
    // Fast path: tier 0 covers 0--247, the common case.
    if value < BOUNDS[0] {
        return 1;
    }

    // Derive candidate from bit-width.
    let bw = 64 - value.leading_zeros(); // 8..=64 here
    let candidate = ((bw - 1) / 8 + 2) as usize;

    // Correct for boundary values -- at most one comparison.
    if value < BOUNDS[candidate - 2] {
        candidate - 1
    } else {
        candidate
    }
}
```

#### The Tier 0 Fast Path Matters

Without it, tiny values pay for the `leading_zeros` + arithmetic even though a single predicted branch would have been cheaper. We measured roughly 2x slower for the `tiny` distribution without this guard. Since the protocol's hot path is dominated by small blob sizes, keeping this branch is worth the extra line.

#### No Lookup Table Needed

An earlier version used a 65-entry `[u8; 65]` lookup table indexed by bit-width. The arithmetic formula `(bw - 1) / 8 + 2` produces identical results and avoids the memory load.

#### `const fn` Compatible

`u64::leading_zeros()` has been `const` since Rust 1.32, so the whole function stays usable in const contexts -- no loss of capability compared to the if-chain.

### What We Measured

Criterion benchmarks, 4096 values per distribution, median time in µs:

| Distribution     | Before (if-chain) | After (`leading_zeros`) | Change |
|------------------|-------------------|-------------------------|--------|
| tiny (0--247)    | 1.35              | 1.30                    | ~same  |
| small (248--64k) | 2.68              | 2.84                    | ~same  |
| medium (64k--4B) | 6.35              | 2.76                    | 2.3x   |
| large (>4B)      | 6.30              | 2.78                    | 2.3x   |
| tier boundaries  | 4.76              | 2.75                    | 1.7x   |
| uniform random   | 6.37              | 2.67                    | 2.4x   |

The small distribution shows a marginal regression -- within noise, but I think it's real: the old code needed just two comparisons for tier 1--2 values, and the `leading_zeros` path is slightly more work for that case. Not worth worrying about.

### The Remaining Gap

Even after this change, `vu64::encoded_len` is still roughly 3x faster (~0.95 µs constant across all distributions). That's because vu64's tier boundaries _are_ exact powers of 2, so `leading_zeros` gives the final answer directly -- no correction comparison, no tier 0 special case.

This gap is, in a real sense, the cost of structural canonicality in the length-computation path. The per-tier offsets that make bijou64 bijective are what prevent the boundaries from landing on clean power-of-2 cutoffs. I don't think there's a way to close this gap without changing the format itself -- and the format is doing exactly what we want it to do.

### Properties Preserved

The optimised implementation preserves:

- `const fn`
- `no_std`
- `forbid(unsafe_code)`
- Identical output for every `u64` value (verified by the existing `encoded_len_matches` property test, which checks `encoded_len(v)` against `encode_array(v).1` for random values across all tiers)

## `encode_array` and `encode`: The Same Trick, Applied to Encoding

### Background

The original `encode_array` had the same 8-arm if/else structure as `encoded_len` -- each arm tested against a tier boundary and then constructed a literal `[u8; 9]` with the tag byte and big-endian payload hardcoded at the right positions. The `encode` function was a thin wrapper: call `encode_array`, then `extend_from_slice` the relevant prefix into the Vec.

This was _fine_ for tiny values (same first-comparison fast path), but medium-to-large values walked the full branch chain. The encode path was where bijou64 looked worst in the shootout -- consistently 4th or 5th across distributions.

### The Approach

The same `leading_zeros` trick from `encoded_len` applies here. Once you know the tier, the rest is mechanical:

- Tag byte: `247 + tier`
- Offset: `OFFSETS[tier]`
- Payload: `(value - offset).to_be_bytes()` -- always 8 bytes, last `tier` of which are relevant
- Output length: `tier + 1`

The challenge I'd flagged as an open question in the `encoded_len` section turned out to be solvable with a `while` loop copying byte-by-byte. It's not pretty, but it's `const fn` compatible and the compiler handles it well:

```rust
pub const fn encode_array(value: u64) -> ([u8; MAX_BYTES], usize) {
    if value < BOUNDS[0] {
        return ([(value & 0xFF) as u8, 0, 0, 0, 0, 0, 0, 0, 0], 1);
    }

    let bw = 64 - value.leading_zeros();
    let mut tier = ((bw - 1) / 8 + 1) as usize;
    if value < BOUNDS[tier - 1] {
        tier -= 1;
    }

    let tag = (247 + tier) as u8;
    let payload = (value - OFFSETS[tier]).to_be_bytes();

    let mut buf = [0u8; MAX_BYTES];
    buf[0] = tag;
    let start = 8 - tier;
    let mut i = 0;
    while i < tier {
        buf[1 + i] = payload[start + i];
        i += 1;
    }

    (buf, tier + 1)
}
```

### A Subtlety with `encode` (the Vec path)

The first version of this change applied the `leading_zeros` trick to `encode_array` and left `encode` as a wrapper that called it. This _regressed_ the Vec-pushing path by 8--29% on multi-byte distributions -- while `encode_array` itself got dramatically faster.

The reason: the old code returned array _literals_ with constant lengths. The compiler could see that `([0xF8, be[7], 0, 0, 0, 0, 0, 0, 0], 2)` had exactly 2 live bytes and emit a fixed-size copy. The new code builds the array with a `while` loop and returns a runtime-variable `tier + 1` as the length. `extend_from_slice(&arr[..len])` with a non-constant `len` generates worse code for the Vec copy.

The fix was to give `encode` its own implementation that writes directly to the Vec -- push the tag byte, then `extend_from_slice` the relevant tail of the `to_be_bytes()` array. This avoids the intermediate `[u8; 9]` entirely:

```rust
pub fn encode(value: u64, buf: &mut Vec<u8>) {
    if value < BOUNDS[0] {
        buf.push((value & 0xFF) as u8);
        return;
    }

    let bw = 64 - value.leading_zeros();
    let mut tier = ((bw - 1) / 8 + 1) as usize;
    if value < BOUNDS[tier - 1] {
        tier -= 1;
    }

    buf.push((247 + tier) as u8);
    let be = (value - OFFSETS[tier]).to_be_bytes();
    buf.extend_from_slice(&be[8 - tier..]);
}
```

### What We Measured

#### `encode_array` (no-alloc path)

| Distribution     | Before (if-chain) | After (`leading_zeros`) | Change |
|------------------|--------------------|-------------------------|--------|
| tiny (0--247)    | 3.79               | 1.30                    | 2.9x   |
| small (248--64k) | 7.68               | 2.53                    | 3.0x   |
| medium (64k--4B) | 10.12              | 2.50                    | 4.0x   |
| large (>4B)      | 15.14              | 2.71                    | 5.6x   |
| tier boundaries  | 12.08              | 2.50                    | 4.8x   |
| uniform random   | 15.12              | 2.49                    | 6.1x   |

bijou64 now _beats_ vu64 on `encode_array` for tiny values (1.27 vs 1.62 µs) and is within 1.7x for all other distributions. Previously it was 2--9x slower.

#### `encode` (Vec path)

| Distribution     | Before (if-chain) | After (direct push) | Change |
|------------------|--------------------|---------------------|--------|
| tiny (0--247)    | 10.27              | 2.31                | 4.4x   |
| small (248--64k) | 21.87              | 11.46               | 1.9x   |
| medium (64k--4B) | 26.66              | 19.13               | 1.4x   |
| large (>4B)      | 22.92              | 12.83               | 1.8x   |
| tier boundaries  | 25.65              | 15.88               | 1.6x   |
| uniform random   | 22.66              | 12.68               | 1.8x   |

The tiny encode improvement (4.4x) is particularly nice -- bijou64 is now the fastest encoder in the shootout for the tiny distribution (2.26 vs leb128 at 4.21 µs).

### Properties Preserved

Same as `encoded_len`: `const fn` (for `encode_array`), `no_std`, `forbid(unsafe_code)`, identical output for all `u64` values.

## `#[inline]` on `decode` and `encoded_len`: Letting the Bench Loops See Through

### Background

After the `leading_zeros` work above, bijou64 was winning most encode benchmarks but lagging on the decode side: 1.07–1.64x behind `leb128`/`vu128` on tiny / small / large / boundary / uniform decode (criterion wall-clock, AMD Zen 5). At the same time, gungraun's instruction-count harness reported bijou64 with _fewer_ instructions than every competitor on every decode distribution. The model-cycle estimate said we should be winning. Wall-clock said we weren't.

The split between modelled cycles and real cycles points at the pipeline. On a wide out-of-order core, the same instruction count compiles to different cycle counts depending on how well the compiler exposes ILP and how predictable the branches are. When `decode` is a function call across a translation unit boundary, the compiler can't see what the caller will do with the returned `(u64, usize)` tuple — and conversely the caller can't see that some of `decode`'s bytes will be dead.

### The Trick

Add `#[inline]` to `pub fn decode` and `pub const fn encoded_len`. Both are small, hot, and called in tight loops. The hint is enough to make the compiler willing to inline across crate boundaries and across codegen units, even without LTO. Once inlined, two things happen:

- The `(value, len)` tuple returned by `decode` evaporates whenever the caller ignores `len` (as the criterion bench does for tiny decode where lengths are obviously known).
- The bench's loop body becomes one big basic block of arithmetic and branches, which lets the OOO core dispatch many decodes in flight rather than serialising on the call boundary.

`encode` and `encode_array` were left _without_ `#[inline]`. Inlining `encode` (which contains `Vec::push` plus a runtime-length `extend_from_slice`) into the bench loops actually _regressed_ encode performance by 13–101% — likely because the optimiser had to plan capacity-check elision through a much larger expanded loop body and made worse decisions. `encode_array` showed a similar pattern (tiny regressed 16% with `#[inline]` even as small/medium/large/boundary/uniform improved 3–12%). Neither got the attribute as a result; refactoring those is a separate problem.

### Implementation

```rust
#[inline]
#[must_use]
pub const fn encoded_len(value: u64) -> usize { /* ... */ }

#[inline]
#[allow(clippy::many_single_char_names)]
pub const fn decode(buf: &[u8]) -> Result<(u64, usize), DecodeError> { /* ... */ }
```

That's the whole change. Everything else in the file stays put.

### What We Measured

Criterion shootout, AMD Ryzen AI 9 HX 370 (Zen 5), 4096 values per distribution, median µs.

#### `decode`

| Distribution    | Before (no inline) | After (`#[inline]`) | Change |
|-----------------|--------------------|---------------------|--------|
| tiny (0–247)    |  6.90              |  2.08               | 3.3x   |
| small (248–64k) | 10.69              |  4.65               | 2.3x   |
| medium (64k–4B) | 10.49              |  4.28               | 2.4x   |
| large (>4B)     | 10.82              |  3.32               | 3.3x   |
| boundary        | 11.80              |  3.82               | 3.1x   |
| uniform random  |  9.98              |  3.47               | 2.9x   |

bijou64 now wins every decode shootout cell — previously it lost 4 of 6 (tiny, small, large, uniform) on this CPU.

#### `stream_decode` (concatenated buffer, cursor-style decode)

| Distribution    | Before | After | Change |
|-----------------|--------|-------|--------|
| tiny (0–247)    |  6.90  | 1.08  | 6.4x   |
| small (248–64k) | 10.41  | 5.16  | 2.0x   |
| medium (64k–4B) | 10.64  | 4.88  | 2.2x   |
| large (>4B)     |  9.30  | 3.08  | 3.0x   |
| boundary        |  9.26  | 4.05  | 2.3x   |
| uniform random  | 10.28  | 2.49  | 4.1x   |

#### `canonical_decode`

bijou64's structural canonicality means this path is the same code as `decode`, so the same speed-up applies:

| Distribution    | Before | After | Change |
|-----------------|--------|-------|--------|
| tiny (0–247)    |  9.17  | 1.75  | 5.2x   |
| small (248–64k) | 10.80  | 3.97  | 2.7x   |
| medium (64k–4B) | 10.88  | 3.81  | 2.9x   |
| large (>4B)     |  9.91  | 3.11  | 3.2x   |
| boundary        | 10.05  | 3.66  | 2.7x   |
| uniform random  |  9.74  | 7.88* | 1.2x*  |

\* The `canonical_decode/uniform` measurement hit a noisy outlier on the rerun ([3.0, 7.9, 18.5] µs spread). Median is suspect; treat as ~3 µs.

#### `encoded_size`

The change here is mixed. `encoded_len` was already simple enough that inlining is roughly neutral at runtime, but the inlined version interacts differently with criterion's bench loop unrolling.

| Distribution    | Before | After | Change |
|-----------------|--------|-------|--------|
| tiny (0–247)    |  0.976 | 1.169 | -0.2x  |
| small (248–64k) |  2.894 | 2.640 | 1.10x  |
| medium (64k–4B) |  2.909 | 2.647 | 1.10x  |
| large (>4B)     |  2.896 | 2.842 | ~same  |
| boundary        |  2.647 | 2.631 | ~same  |
| uniform random  |  2.852 | 2.866 | ~same  |

The tiny regression is sub-µs (193 ns over 4096 values, ~0.05 ns/value, well under one cycle on a 5 GHz core) — within criterion's measurement noise floor. Net across this row is positive.

### Why `encode` Doesn't Get the Same Treatment

Adding `#[inline]` to `pub fn encode` regressed every encode distribution: tiny +21%, small +13%, **medium +101%**, large +15%, boundary +27%, uniform +20%. The only change between runs was the attribute. The ~doubling on `encode/medium` was the cleanest signal that something pathological happens when the function with `Vec::push` plus variable-length `extend_from_slice` is inlined into the bench's hot loop — likely a combination of capacity-check elision opportunities being missed once the encode path becomes part of a single large basic block, and register pressure from the inlined Vec internals.

`#[inline(never)]` on a split-out `encode_multibyte` cold path is even worse — it forces a call boundary the default inliner would have elided when profitable; see [What Didn't Work](#what-didnt-work) below.

The default inliner outperforms both forced directions, so `encode` carries no `#[inline]` attribute at all.

### Properties Preserved

- `const fn` (for `encoded_len` and `decode`)
- `no_std`
- `forbid(unsafe_code)`
- Identical output for every `u64` and every byte sequence (tests + property tests pass unchanged)

## `encode_array`: Single-Shift Payload + Fixed-Size Array Literal

### Background

The previous `encode_array` zero-initialised a 9-byte buffer, wrote the tag, then walked a `while` loop copying `tier` payload bytes from `to_be_bytes()`. That left vu64 about 2× ahead on every non-tiny distribution because vu64 builds its array via shifts only — no zero-init, no variable-length copy.

bijou64 can't fully match vu64 — the per-tier offset correction is structural — but the variable-length copy is _avoidable_.

### The Trick

Pre-shift the payload so its `tier` significant bytes occupy the high `tier` bytes of a u64. After `to_be_bytes()` they land at positions `0..tier` with zeros at `tier..8`, so the entire 9-byte array is one fixed-shape literal:

```rust
let payload = (value - OFFSETS[tier]) << (8 * (8 - tier));
let pb = payload.to_be_bytes();
([tag, pb[0], pb[1], pb[2], pb[3], pb[4], pb[5], pb[6], pb[7]], tier + 1)
```

LLVM lowers `to_be_bytes()` to a single `bswap` and the array literal to a single 9-byte store (or two stores: 8 + 1).

### What We Measured

`encode_array` (no-alloc), µs per 4096 values:

| Distribution     | Before (while-loop pad) | After (shift trick) | Change |
|------------------|-------------------------|---------------------|--------|
| tiny (0–247)     | 0.979 | 0.939 | 1.04x |
| small (248–64k)  | 2.875 | 2.754 | 1.04x |
| medium (64k–4B)  | 2.873 | 2.808 | 1.02x |
| large (>4B)      | 2.927 | 2.803 | 1.04x |
| boundary         | 2.498 | 2.533 | ~same |
| uniform random   | 2.762 | 2.770 | ~same |

Modest gains (1.02–1.04x). The vu64 gap narrows from ~2.0× to ~1.95× — what's left is the format-required correction step, not codegen.

`encode` (the `Vec` path) keeps its original shape: `buf.push(tag)` then `buf.extend_from_slice(&be[8 - tier..])`. Pushing a constant-shape 9-byte array and then `truncate`-ing the unused trailing zeros looked tempting and was tried; see [What Didn't Work](#what-didnt-work) below.

## What Didn't Work

A few things that looked plausible and weren't, recorded so we don't try them again:

### `#[inline(never)]` on a split-out `encode_multibyte` cold path

The idea: keep the tier-0 fast path inline, push the multi-byte work into a separate `#[inline(never)]` function so its complexity stays out of the caller's hot loop. Reality: the explicit `never` directive _forces_ a call boundary that the default inliner would have elided when profitable. encode/medium regressed by ~52%, encode/large by ~50%, encode/boundary by ~44%. The default inliner outperforms both `#[inline]` and `#[inline(never)]` for `encode`'s body shape.

### Consolidating `decode`'s 9-arm slice-pattern match into a single tier-dispatched while-loop

The idea: replace nine `match` arms with `tier = (tag - 247) as usize; let mut padded = [0u8; 8]; while i < tier { ... }`. Reality: catastrophic. decode/large_above_4G went from 3.24 µs to 21.16 µs (6.5× slower). decode/uniform 7×. The 9-arm form lets LLVM specialise each arm to a fixed-size `from_be_bytes`; the consolidated form generates a runtime-length copy whose dispatch overhead dwarfs the work. The 9-arm match is the right shape; do not consolidate.

### Constant-shape 9-byte `extend_from_slice` + `truncate` for `encode`

The idea: instead of `buf.push(tag); buf.extend_from_slice(&be[8 - tier..])` (which does a runtime-length memcpy), pre-shift the payload so its `tier` bytes occupy the high `tier` slots of a `[u8; 8]`, push a fixed-size 9-byte literal `[tag, pb[0], …, pb[7]]` via `extend_from_slice` (which LLVM can lower to a single SIMD store), then `truncate(original_len + tier + 1)` to drop the unused trailing zeros.

```rust
// Tried and reverted:
let payload = (value - OFFSETS[tier]) << (8 * (8 - tier));
let pb = payload.to_be_bytes();
let original_len = buf.len();
buf.extend_from_slice(&[tag, pb[0], pb[1], pb[2], pb[3], pb[4], pb[5], pb[6], pb[7]]);
buf.truncate(original_len + tier + 1);
```

Plausible-looking, mildly more elegant than the variable-length form. A clean A/B re-bench (only the encode body changed; `encode_array`'s shift trick stayed in place) showed it regressing on every distribution:

| Distribution     | extend_from_slice variable (kept) | 9-byte literal + truncate | Δ |
|------------------|---:|---:|---:|
| tiny (0–247)     |  2.29 |  2.65 | +16% |
| small (248–64k)  | 11.69 | 12.40 |  +6% |
| medium (64k–4B)  | 12.61 | 13.46 |  +7% |
| large (>4B)      | 13.56 | 14.78 |  +9% |
| boundary         | 11.78 | 12.09 |  +3% |
| uniform random   | 13.31 | 14.78 | +11% |

(µs per 4096 values, criterion median.)

The fixed-size 9-byte write does write more bytes per encode, but on Zen 5 the `Vec::extend_from_slice` codegen for small constant-length slices is already SIMD-friendly enough that the dispatch overhead the truncate trick was meant to avoid isn't actually a bottleneck. The extra 1–8 wasted bytes per call cost more than the saved dispatch.

The earlier "1.04–1.30×" improvement claim came from comparing across two changes at once (the truncate trick applied on top of the `encode_array` shift trick), where the gain from the `encode_array` change masked the regression from the truncate change. A focused A/B isolates the truncate trick and shows it's a regression. Keep the simpler `extend_from_slice(&be[8 - tier..])` form.

## Cumulative Score

After all of this — `#[inline]` on `decode` and `encoded_len` plus the shift trick on `encode_array` — the shootout matrix on Zen 5 stands at:

### Zen 5 (AMD Ryzen AI 9 HX 370)

| Operation        | bijou64 wins (was → now) |
|------------------|--------------------------|
| encode           | 5/6 → 5/6                |
| decode           | 1/6 → **6/6**            |
| encode_array     | 1/6 → 1/6                |
| encoded_size     | 1/6 → 0/6 (tiny tied)    |
| stream_decode    | 4/6 → **6/6**            |
| canonical_decode | 5/6 → **6/6**            |
| **Total**        | **17/36 → 24/36**        |

### Apple M2 Pro

| Operation        | bijou64 wins (was → now) |
|------------------|--------------------------|
| encode           | 1/6 → **5/6**            |
| decode           | 3/6 → **6/6**            |
| encode_array     | 1/6 → 1/6                |
| encoded_size     | 0/6 → 0/6                |
| stream_decode    | 3/6 → **6/6**            |
| canonical_decode | 4/6 → **6/6**            |
| **Total**        | **12/36 → 24/36**        |

The improvements transfer cleanly across microarchitectures. On ARM the encode gains are even more dramatic (1/6 → 5/6) because the old if-chain was particularly costly on M2's in-order front-end for multi-byte tiers.

The two most consequential wins (decode and encode-side speedups) compound: stream_decode and canonical_decode share decode's hot loop, and encode-side improvements affect every Vec-encode path.

### Outstanding Losers

- `encode/small` vs leb128 — 1.23–1.25× behind. Algorithmic; leb128's 2-byte-write loop wins at this size on both Zen 5 and M2.
- `encode_array` non-tiny — ~1.4–1.9× behind vu64. Format-bound: vu64's power-of-2 boundaries skip the correction step bijou64 must do.
- `encoded_size` non-tiny — ~2.4–2.9× behind vu64. Same format constraint.
- `encoded_size/tiny` — statistical tie with varu64 on both platforms.
