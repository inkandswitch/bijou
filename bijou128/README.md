# 💎 bijou128

Bijective variable-length encoding for unsigned 128-bit integers.

Pronounced "bee-zoo-one-twenty-eight" — **bij**ective **o**ffset **u128**.

`bijou128` encodes `u128` values into 1–17 bytes using a tag-byte prefix
scheme derived from [VARU64], modified with per-tier offsets to achieve
_structural canonicality_ — each value has exactly one encoding, and
each encoding has exactly one value.

It is the 128-bit sibling of [`bijou64`]. The format is the same family
with two structural differences: the tag threshold is **240** (vs 248 in
`bijou64`), and there are **16 multi-byte tiers** (vs 8). All 16 tiers
fit in single-byte tags `0xF0..=0xFF`, so length-from-first-byte still
holds — no extended framing needed.

## Quick start

```rust
// Encode
let mut buf = Vec::new();
bijou128::encode(300, &mut buf);
assert_eq!(buf, [0xF0, 0x3C]); // tag 0xF0, payload 300 - 240 = 60

// Decode
let (value, len) = bijou128::decode(&buf).unwrap();
assert_eq!(value, 300);
assert_eq!(len, 2);

// Stack-allocated encoding (no alloc needed)
let (bytes, len) = bijou128::encode_array(300);
assert_eq!(&bytes[..len], &[0xF0, 0x3C]);

// Query encoded length without encoding
assert_eq!(bijou128::encoded_len(300), 2);
```

## Encoding

| First byte  | Total length | Offset     | Value range                |
|-------------|--------------|------------|----------------------------|
| 0x00 – 0xEF | 1            | 0          | 0 – 239                    |
| 0xF0        | 2            | 240        | 240 – 495                  |
| 0xF1        | 3            | 496        | 496 – 66,031               |
| 0xF2        | 4            | 66,032     | 66,032 – 16,843,247        |
| 0xF3        | 5            | 16,843,248 | 16,843,248 – 4,311,810,543 |
| 0xF4 – 0xFE | 6 – 16       | ...        | ...                        |
| 0xFF        | 17           | OFFSETS[16]| ... – u128::MAX            |

Values below 240 encode as a single byte equal to the value. Larger
values use a tag byte (`0xF0`–`0xFF`) followed by 1–16 big-endian
payload bytes encoding `value - OFFSET[tier]`.

See [SPEC.md](SPEC.md) for the full specification, offset table,
worked examples, and test vectors.

## Features

- `no_std` (requires `alloc` for `encode()`; `encode_array()` and
  `decode()` are allocation-free)
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

## Relationship to `bijou64`

The format is _not_ wire-compatible with `bijou64`. The tag threshold
is different (240 vs 248) and a value's encoding will differ between
the two crates. Pick `bijou64` if you know your values fit in `u64`;
pick `bijou128` if you need the wider range.

## License

The code is licensed under MIT OR Apache-2.0 (workspace default).
The [specification](SPEC.md) is licensed under
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

[VARU64]: https://github.com/AljoschaMeyer/varu64-rs
[`bijou64`]: https://crates.io/crates/bijou64
