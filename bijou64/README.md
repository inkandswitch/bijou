# 💎 bijou64

Bijective variable-length encoding for unsigned 64-bit integers.

Pronounced "bee-zoo-sixty-four" — **bij**ective **o**ffset **u64**.

`bijou64` encodes `u64` values into 1–9 bytes using a tag-byte prefix
scheme derived from [VARU64], modified with per-tier offsets to achieve
_structural canonicality_ — each value has exactly one encoding, and
each encoding has exactly one value.

## Quick start

```rust
// Encode — appends 1..=9 bytes to the existing buffer.
let mut buf = Vec::new();
bijou64::encode(300, &mut buf);
assert_eq!(buf, [0xF8, 0x34]); // tag 0xF8, payload 300 - 248 = 52

// Decode — returns the value and the number of bytes consumed,
// leaving any trailing bytes untouched.
let (value, len) = bijou64::decode(&buf).unwrap();
assert_eq!(value, 300);
assert_eq!(len, 2);

// Stack-allocated encoding (no alloc needed). The returned value
// derefs to `&[u8]` of the correct length — no slicing required.
let bytes = bijou64::encoded_bytes(300);
assert_eq!(&*bytes, &[0xF8, 0x34]);

// Query encoded length without encoding
assert_eq!(bijou64::encoded_len(300), 2);
```

`encode` _appends_ to its buffer rather than overwriting, so you can
build up a stream of encoded values back-to-back:

```rust
let mut buf = Vec::new();
for value in [0u64, 42, 248, 65_535, u64::MAX] {
    bijou64::encode(value, &mut buf);
}
// buf now contains five concatenated bijou64 encodings.
```

## Streaming decode

Each `decode` call returns `(value, consumed_bytes)`. To read a
sequence of back-to-back values, advance the cursor by `consumed_bytes`
after each call:

```rust
let mut cursor: &[u8] = &buf;
let mut decoded = Vec::new();
while !cursor.is_empty() {
    let (value, n) = bijou64::decode(cursor).unwrap();
    decoded.push(value);
    cursor = &cursor[n..];
}
```

If you'd rather use iterator combinators, `decode_iter` returns a
fused `Iterator<Item = Result<u64, DecodeError>>`:

```rust
let total: u64 = bijou64::decode_iter(&buf).filter_map(Result::ok).sum();
```

Or, to get every value or the first error in one call:

```rust
let values: Result<Vec<u64>, _> = bijou64::decode_all(&buf);
```

See [`examples/decode64.rs`](./examples/decode64.rs) for a runnable
demonstration of all three patterns.

## Encoding

| First byte  | Total length | Offset     | Value range                |
|-------------|--------------|------------|----------------------------|
| 0x00 – 0xF7 | 1            | 0          | 0 – 247                    |
| 0xF8        | 2            | 248        | 248 – 503                  |
| 0xF9        | 3            | 504        | 504 – 66,039               |
| 0xFA        | 4            | 66,040     | 66,040 – 16,843,255        |
| 0xFB        | 5            | 16,843,256 | 16,843,256 – 4,311,810,551 |
| 0xFC – 0xFF | 6 – 9        | ...        | ... – u64::MAX             |

Values below 248 encode as a single byte equal to the value. Larger
values use a tag byte (`0xF8`–`0xFF`) followed by 1–8 big-endian
payload bytes encoding `value - OFFSET[tier]`.

See [SPEC.md](SPEC.md) for the full specification, offset table,
worked examples, and test vectors.

## Features

- `no_std` (requires `alloc` for `encode()` and `decode_all()`;
  `encoded_bytes()` and `decode()` are allocation-free)
- `#![forbid(unsafe_code)]`
- Canonical by construction — no runtime canonicality checks
- Big-endian payloads — lexicographic byte order = numeric order
- Total encoding length determined from first byte alone
- Full `u64` range (0 to 2^64 − 1)

## Optional features

| Feature     | Description                                              |
|-------------|----------------------------------------------------------|
| `arbitrary` | `Arbitrary` impl for fuzz testing                        |
| `bolero`    | Property-based testing with bolero (implies `arbitrary`) |

## Family

bijou64 is one of three width-specialised siblings sharing the same
recurrence, big-endian payload layout, and canonical-by-construction
property. They differ only in the tag-byte threshold and tier count:

- [`bijou32`](../bijou32) — narrower `u32` variant (1–5 bytes, threshold `252`).
- [`bijou128`](../bijou128) — wider `u128` variant (1–17 bytes, threshold `240`).

## License

The code is licensed under MIT OR Apache-2.0 (workspace default).
The [specification](SPEC.md) is licensed under
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

[VARU64]: https://github.com/AljoschaMeyer/varu64-rs
