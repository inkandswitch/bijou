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
| [`bijou64`](./bijou64)                 | `u64`   | 9         | The reference implementation. Tag threshold 248.           |
| [`bijou64_wasm`](./bijou64_wasm)       | `u64`   | 9         | Wasm/JS bindings for `bijou64` (npm package `bijou64`).    |
| [`bijou128`](./bijou128)               | `u128`  | 17        | Same scheme widened to 128 bits. Tag threshold 240.        |
| [`bijou128_wasm`](./bijou128_wasm)     | `u128`  | 17        | Wasm/JS bindings for `bijou128` (npm package `bijou128`).  |

The 64- and 128-bit variants are **not wire-compatible** — they use
different tag thresholds (248 vs 240) so they can both reach their
respective maximums in the smallest number of bytes. Pick the width
that matches your value domain; don't mix encodings on the same wire
without an out-of-band signal.

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

For 128-bit values, the API is identical:

```rust
let mut buf = Vec::new();
bijou128::encode(500, &mut buf);
assert_eq!(buf, [0xF1, 0x00, 0x04]);

let (value, len) = bijou128::decode(&buf).unwrap();
assert_eq!(value, 500);
assert_eq!(len, 3);
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

## License

Code is dual-licensed under [MIT](./LICENSE-MIT) OR
[Apache-2.0](./LICENSE-APACHE).
The encoding specifications (`SPEC.md` in each crate) are licensed
under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
