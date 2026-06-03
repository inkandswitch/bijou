# 💎 bijou128

Bijective variable-length encoding for unsigned 128-bit integers.

Pronounced "bee-zoo-one-twenty-eight" — **bij**ective **o**ffset **u128**.

`bijou128` is the wider sibling of [`bijou64`](../bijou64). Same recurrence,
same big-endian payload layout, same canonical-by-construction property,
extended to cover the full `u128` range with 1–17 bytes per value.

## Quick start

```rust
// Encode — appends 1..=17 bytes to the existing buffer.
let mut buf = Vec::new();
bijou128::encode(500, &mut buf);
assert_eq!(buf, [0xF1, 0x00, 0x04]); // tag 0xF1, payload 500 - 496 = 4

// Decode — returns the value and the number of bytes consumed,
// leaving any trailing bytes untouched.
let (value, len) = bijou128::decode(&buf).unwrap();
assert_eq!(value, 500);
assert_eq!(len, 3);

// Stack-allocated encoding (no alloc needed). The returned value
// derefs to `&[u8]` of the correct length — no slicing required.
let bytes = bijou128::encoded_bytes(500);
assert_eq!(&*bytes, &[0xF1, 0x00, 0x04]);

// Query encoded length without encoding.
assert_eq!(bijou128::encoded_len(500), 3);
```

`encode` _appends_ to its buffer rather than overwriting, so you can
build up a stream of encoded values back-to-back:

```rust
let mut buf = Vec::new();
for value in [0u128, 42, 240, 65_535, u128::MAX] {
    bijou128::encode(value, &mut buf);
}
// buf now contains five concatenated bijou128 encodings.
```

## Streaming decode

Each `decode` call returns `(value, consumed_bytes)`. To read a
sequence of back-to-back values, advance the cursor by `consumed_bytes`
after each call:

```rust
let mut cursor: &[u8] = &buf;
let mut decoded = Vec::new();
while !cursor.is_empty() {
    let (value, n) = bijou128::decode(cursor).unwrap();
    decoded.push(value);
    cursor = &cursor[n..];
}
```

If you'd rather use iterator combinators, `decode_iter` returns a
fused `Iterator<Item = Result<u128, DecodeError>>`:

```rust
let total: u128 = bijou128::decode_iter(&buf).filter_map(Result::ok).sum();
```

Or, to get every value or the first error in one call:

```rust
let values: Result<Vec<u128>, _> = bijou128::decode_all(&buf);
```

See [`examples/decode128.rs`](./examples/decode128.rs) for a runnable
demonstration of all three patterns.

## Encoding

| First byte  | Total length | Offset            | Value range                  |
|-------------|--------------|-------------------|------------------------------|
| 0x00 – 0xEF | 1            | 0                 | 0 – 239                      |
| 0xF0        | 2            | 240               | 240 – 495                    |
| 0xF1        | 3            | 496               | 496 – 66,031                 |
| 0xF2        | 4            | 66,032            | 66,032 – 16,843,247          |
| 0xF3        | 5            | 16,843,248        | 16,843,248 – 4,311,810,543   |
| 0xF4 – 0xFF | 6 – 17       | …                 | … – u128::MAX                |

Values below 240 encode as a single byte equal to the value. Larger
values use a tag byte (`0xF0`–`0xFF`) followed by 1–16 big-endian
payload bytes encoding `value - OFFSET[tier]`.

## Comparison with bijou32 and bijou64

|                  | bijou32     | bijou64    | bijou128       |
|------------------|-------------|------------|----------------|
| Integer type     | `u32`       | `u64`      | `u128`         |
| Max bytes        | 5           | 9          | 17             |
| Tag threshold    | 252         | 248        | 240            |
| Multi-byte tags  | 0xFC–0xFF   | 0xF8–0xFF  | 0xF0–0xFF      |
| Multi-byte tiers | 4           | 8          | 16             |
| Tier 0 width     | 0 – 251     | 0 – 247    | 0 – 239        |

None of the three formats are wire-compatible — they use different tag
thresholds so each can reach its maximum in the smallest number of
bytes. bijou64's `0xF7` is a plain value byte (247), but bijou128's
`0xF7` is a tag (tier-8). Pick the width that matches your value
domain; don't mix encodings on the same wire without an out-of-band
signal.

## Features

- `no_std` (requires `alloc` for `encode()` and `decode_all()`;
  `encoded_bytes()` and `decode()` are allocation-free)
- `#![forbid(unsafe_code)]`
- Canonical by construction — no runtime canonicality checks
- Big-endian payloads — lexicographic byte order = numeric order
- Total encoding length determined from first byte alone
- Full `u128` range (0 to 2^128 − 1)

## Optional features

| Feature     | Description                                              |
|-------------|----------------------------------------------------------|
| `arbitrary` | `Arbitrary` impl for fuzz testing                        |
| `bolero`    | Property-based testing with bolero (implies `arbitrary`) |

## Performance

bijou128 shares its core algorithm (per-tier offsets, `leading_zeros`
dispatch, fixed-shape big-endian payload write) with bijou64. The
bijou64 crate is the canonical performance reference: benchmark
methodology, comparison against `leb128` / `varu64` / `vu64` /
`vu128`, optimisation rationale, and the on-disk encoded-size
analysis all live there:

- [`bijou64/SHOOTOUT_ANALYSIS.md`](../bijou64/SHOOTOUT_ANALYSIS.md)
- [`bijou64/OPTIMISATION.md`](../bijou64/OPTIMISATION.md)
- [`bijou64/SIZE_ANALYSIS.md`](../bijou64/SIZE_ANALYSIS.md)

Dedicated bijou128 benchmarks are planned but not yet in this
repository.

## Family

bijou128 is one of three width-specialised siblings sharing the same
recurrence, big-endian payload layout, and canonical-by-construction
property. They differ only in the tag-byte threshold and tier count:

- [`bijou32`](../bijou32) — narrower `u32` variant (1–5 bytes, threshold `252`).
- [`bijou64`](../bijou64) — `u64` variant (1–9 bytes, threshold `248`).

## License

The code is licensed under MIT OR Apache-2.0 (workspace default).
