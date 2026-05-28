# 💎 bijou32

Bijective variable-length encoding for unsigned 32-bit integers.

Pronounced "bee-zoo-thirty-two" — **bij**ective **o**ffset **u32**.

`bijou32` is the narrower sibling of [`bijou64`](../bijou64). Same recurrence,
same big-endian payload layout, same canonical-by-construction property,
sized for the `u32` value range with 4 multi-byte tiers (1–5 bytes per
value).

## Quick start

```rust
// Encode — appends 1..=5 bytes to the existing buffer.
let mut buf = Vec::new();
bijou32::encode(300, &mut buf);
assert_eq!(buf, [0xFC, 0x30]); // tag 0xFC, payload 300 - 252 = 48

// Decode — returns the value and the number of bytes consumed,
// leaving any trailing bytes untouched.
let (value, len) = bijou32::decode(&buf).unwrap();
assert_eq!(value, 300);
assert_eq!(len, 2);

// Stack-allocated encoding (no alloc needed). The returned value
// derefs to `&[u8]` of the correct length — no slicing required.
let bytes = bijou32::encoded_bytes(300);
assert_eq!(&*bytes, &[0xFC, 0x30]);

// Query encoded length without encoding.
assert_eq!(bijou32::encoded_len(300), 2);
```

`encode` _appends_ to its buffer rather than overwriting, so you can
build up a stream of encoded values back-to-back:

```rust
let mut buf = Vec::new();
for value in [0u32, 42, 252, 65_535, u32::MAX] {
    bijou32::encode(value, &mut buf);
}
// buf now contains five concatenated bijou32 encodings.
```

## Streaming decode

Each `decode` call returns `(value, consumed_bytes)`. To read a
sequence of back-to-back values, advance the cursor by `consumed_bytes`
after each call:

```rust
let mut cursor: &[u8] = &buf;
let mut decoded = Vec::new();
while !cursor.is_empty() {
    let (value, n) = bijou32::decode(cursor).unwrap();
    decoded.push(value);
    cursor = &cursor[n..];
}
```

If you'd rather use iterator combinators, `decode_iter` returns a
fused `Iterator<Item = Result<u32, DecodeError>>`:

```rust
let total: u32 = bijou32::decode_iter(&buf).filter_map(Result::ok).sum();
```

Or, to get every value or the first error in one call:

```rust
let values: Result<Vec<u32>, _> = bijou32::decode_all(&buf);
```

See [`examples/decode.rs`](./examples/decode.rs) for a runnable
demonstration of all three patterns.

## Encoding

| First byte  | Total length | Offset     | Value range                       |
|-------------|--------------|------------|-----------------------------------|
| 0x00 – 0xFB | 1            | 0          | 0 – 251                           |
| 0xFC        | 2            | 252        | 252 – 507                         |
| 0xFD        | 3            | 508        | 508 – 66,043                      |
| 0xFE        | 4            | 66,044     | 66,044 – 16,843,259               |
| 0xFF        | 5            | 16,843,260 | 16,843,260 – u32::MAX             |

Values below 252 encode as a single byte equal to the value. Larger
values use a tag byte (`0xFC`–`0xFF`) followed by 1–4 big-endian
payload bytes encoding `value - OFFSET[tier]`.

## Comparison with bijou64 and bijou128

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
bytes. Pick the width that matches your value domain; don't mix
encodings on the same wire without an out-of-band signal.

## Features

- `no_std` (requires `alloc` for `encode()` and `decode_all()`;
  `encoded_bytes()` and `decode()` are allocation-free)
- `#![forbid(unsafe_code)]`
- Canonical by construction — no runtime canonicality checks
- Big-endian payloads — lexicographic byte order = numeric order
- Total encoding length determined from first byte alone
- Full `u32` range (0 to 2^32 − 1)

## Optional features

| Feature     | Description                                              |
|-------------|----------------------------------------------------------|
| `arbitrary` | `Arbitrary` impl for fuzz testing                        |
| `bolero`    | Property-based testing with bolero (implies `arbitrary`) |

## Performance

bijou32 shares its core algorithm (per-tier offsets, `leading_zeros`
dispatch, fixed-shape big-endian payload write) with bijou64. The
bijou64 crate is the canonical performance reference: benchmark
methodology, comparison against `leb128` / `varu64` / `vu64` /
`vu128`, optimisation rationale, and the on-disk encoded-size
analysis all live there:

- [`bijou64/SHOOTOUT_ANALYSIS.md`](../bijou64/SHOOTOUT_ANALYSIS.md)
- [`bijou64/OPTIMISATION.md`](../bijou64/OPTIMISATION.md)
- [`bijou64/SIZE_ANALYSIS.md`](../bijou64/SIZE_ANALYSIS.md)

Dedicated bijou32 benchmarks are planned but not yet in this
repository.

## License

The code is licensed under MIT OR Apache-2.0 (workspace default).
