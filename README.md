# 💎 bijou

Bijective variable-length encodings for unsigned integers.

`bijou` is a family of canonical, length-prefixed varints with the
property that _every value has exactly one encoding_, and _every
encoding decodes to exactly one value_. The tag byte alone determines
the total length, payloads are big-endian (so lexicographic byte
order matches numeric order), and there is no `std` requirement.

## Crates

| Crate                      | Width  | Status      |
|----------------------------|--------|-------------|
| [`bijou64`](./bijou64)     | `u64`  | Released    |
| [`bijou128`](./bijou128)   | `u128` | Released    |
| `bijou32` _(coming soon)_  | `u32`  | Planned     |

Each crate is independently versioned and `no_std`-compatible.
See the per-crate README and `SPEC.md` for the full encoding
specification, offset tables, and test vectors.

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
