# 💎 Bijou

Bijective variable-length encodings for unsigned integers.

`bijou` is a family of canonical, length-prefixed varints with the
property that _every value has exactly one encoding_, and _every
encoding decodes to exactly one value_. The tag byte alone determines
the total length, payloads are big-endian (so lexicographic byte
order matches numeric order), and there is no `std` requirement.

## Crates

| Crate                                  | Integer | Max bytes | Description                                                |
|----------------------------------------|---------|-----------|------------------------------------------------------------|
| [`bijou32`](./bijou32)                 | `u32`   | 5         | The narrowest variant. Tag threshold 252.                  |
| [`bijou32_wasm`](./bijou32_wasm)       | `u32`   | 5         | Wasm/JS bindings for `bijou32` (npm package `bijou32`).    |
| [`bijou64`](./bijou64)                 | `u64`   | 9         | The reference implementation. Tag threshold 248.           |
| [`bijou64_wasm`](./bijou64_wasm)       | `u64`   | 9         | Wasm/JS bindings for `bijou64` (npm package `bijou64`).    |
| [`bijou128`](./bijou128)               | `u128`  | 17        | Same scheme widened to 128 bits. Tag threshold 240.        |
| [`bijou128_wasm`](./bijou128_wasm)     | `u128`  | 17        | Wasm/JS bindings for `bijou128` (npm package `bijou128`).  |

The three width variants are **not wire-compatible** — they use
different tag thresholds (252 vs 248 vs 240) so each can reach its
maximum in the smallest number of bytes. Pick the width that matches
your value domain; don't mix encodings on the same wire without an
out-of-band signal.

The wasm crates also differ in their JS boundary type:

- `bijou32` uses plain JS `number` (since `u32::MAX < Number.MAX_SAFE_INTEGER`)
- `bijou64` and `bijou128` use `bigint`
- `decodeAll` returns `Uint32Array` (bijou32), `BigUint64Array` (bijou64), or `Array<bigint>` (bijou128, since the web platform has no `BigUint128Array`)

## Quick start

```rust
// Encode
let mut buf = Vec::new();
bijou64::encode(300, &mut buf);
assert_eq!(buf, [0xF8, 0x34]);

// Decode
let (value, len) = bijou64::decode(&buf).unwrap();
assert_eq!(value, 300);
assert_eq!(len, 2);
```

For 32-bit and 128-bit values, the API is identical:

```rust
// 32-bit
let mut buf = Vec::new();
bijou32::encode(300, &mut buf);
assert_eq!(buf, [0xFC, 0x30]);

// 128-bit
let mut buf = Vec::new();
bijou128::encode(500, &mut buf);
assert_eq!(buf, [0xF1, 0x00, 0x04]);
```

## Development

This repository is a Cargo workspace with a Nix flake providing the
toolchain and dev tooling.

```sh
nix develop          # enter the dev shell (prints a command menu)
build                # cargo build --workspace
test                 # cargo test --workspace
ci                   # fmt + clippy + test + no_std + wasm32
bench:shootout       # criterion shootout vs other varints
bench:gungraun       # gungraun instruction-count benchmarks
```

Without Nix:

```sh
cargo build --workspace
cargo test --workspace --all-features
```

The workspace targets stable Rust (see `rust-version` in
`Cargo.toml`) and supports `wasm32-unknown-unknown` via the
toolchain shipped in the flake.

## Other implementations

The community has ported the `bijou64` wire format to several languages and ecosystems. These are
independent projects, not maintained here — but they target the same `bijou64`
format, so an encoder in one language interoperates with a decoder in
another:

| Implementation                                                                                | Language          | Notes                                                                |
|-----------------------------------------------------------------------------------------------|-------------------|----------------------------------------------------------------------|
| [LostKobrakai/bijou64](https://github.com/LostKobrakai/bijou64)                               | Elixir            | `bijou64` port ([published on Hex](https://hex.pm/packages/bijou64)) |
| [MichaelMure/go-bijou](https://github.com/MichaelMure/go-bijou)                               | Go                | `bijou64` port                                                       |
| [scottchiefbaker/perl-Encode-Bijou64](https://github.com/scottchiefbaker/perl-Encode-Bijou64) | Perl              | `bijou64` ([on CPAN](https://metacpan.org/dist/Encode-Bijou64))      |
| [Joel-hanson/bijou64](https://github.com/Joel-hanson/bijou64)                                 | Java (+ Rust JNI) | `bijou64` Kafka serializer/deserializer                              |

Building another one? Open a PR adding it here! The per-crate `SPEC.md`
files and the test vectors in each crate's test suite are the reference
for compatibility.

## License

Code is dual-licensed under [MIT](./LICENSE-MIT) OR
[Apache-2.0](./LICENSE-APACHE).
The encoding specifications (`SPEC.md` in each crate) are licensed
under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
