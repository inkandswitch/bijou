# 💎 Bijou

Bijective variable-length encodings for unsigned integers.

`bijou` is a family of canonical, length-prefixed varints with the
property that _every value has exactly one encoding_, and _every
encoding decodes to exactly one value_. The tag byte alone determines
the total length, payloads are big-endian (so lexicographic byte
order matches numeric order), and there is no `std` requirement.

## Crates

Everything ships from two crates:

| Crate                          | Contents                                                                 |
|--------------------------------|--------------------------------------------------------------------------|
| [`bijoux`](./bijoux)           | All format implementations + `Encode`/`Decode` traits on the integers    |
| [`bijoux_wasm`](./bijoux_wasm) | Wasm/JS bindings, one npm package (`@inkandswitch/bijoux`)               |

Within `bijoux`, each width is a module gated by a feature (all enabled
by default):

| Module / feature       | Integer | Max bytes | Tag threshold | Spec                                 |
|------------------------|---------|-----------|---------------|--------------------------------------|
| `bijoux::i32`, feature `i32`  | `i32`   | 5         | 252 (zigzag)  | [specs/bijou32s.md](./specs/bijou32s.md) |
| `bijoux::i64`, feature `i64`  | `i64`   | 9         | 248 (zigzag)  | [specs/bijou64s.md](./specs/bijou64s.md) |
| `bijoux::i128`, feature `i128` | `i128`  | 17        | 240 (zigzag)  | [specs/bijou128s.md](./specs/bijou128s.md) |
| `bijoux::u32`, feature `u32`  | `u32`   | 5         | 252           | [specs/bijou32.md](./specs/bijou32.md)   |
| `bijoux::u64`, feature `u64`  | `u64`   | 9         | 248           | [specs/bijou64.md](./specs/bijou64.md)   |
| `bijoux::u128`, feature `u128` | `u128`  | 17        | 240           | [specs/bijou128.md](./specs/bijou128.md) |

The width variants are **not wire-compatible** — they use different tag
thresholds so each can reach its maximum in the smallest number of
bytes. Pick the width that matches your value domain; don't mix
encodings on the same wire without an out-of-band signal.

On the JS side, exports are width-suffixed (`encodeU64`, `decodeU32`,
…); `u32` uses plain `number`, `u64`/`u128` use `bigint`, and
`decodeAll*` returns `Uint32Array` / `BigUint64Array` /
`Array<bigint>` respectively (the web platform has no
`BigUint128Array`).

> [!NOTE]
> The previous per-width crates (`bijou32`, `bijou64` 0.2.x,
> `bijou128`) are superseded by `bijoux`. Migrate by depending on
> `bijoux` and prefixing paths (`bijou64::encode` →
> `bijoux::u64::encode`), or use the traits.

## Quick start

```rust
use bijoux::{Decode, Encode};

// Encode: traits are implemented directly on the integer types
let mut buf = Vec::new();
300u64.encode(&mut buf);
assert_eq!(buf, [0xF8, 0x34]);

// Decode
let (value, len) = u64::decode(&buf).unwrap();
assert_eq!(value, 300);
assert_eq!(len, 2);
```

Free functions per width are available too, and the API is identical
across widths:

```rust
// 64-bit
let mut buf = Vec::new();
bijoux::u64::encode(300, &mut buf);
assert_eq!(buf, [0xF8, 0x34]);

// 32-bit
let mut buf = Vec::new();
bijoux::u32::encode(300, &mut buf);
assert_eq!(buf, [0xFC, 0x30]);

// 128-bit
let mut buf = Vec::new();
bijoux::u128::encode(500, &mut buf);
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

Building another one? Open a PR adding it here! The specs in
[`specs/`](./specs) and the test vectors in the test
suite are the reference for compatibility.

## License

Code is dual-licensed under [MIT](./LICENSE-MIT) OR
[Apache-2.0](./LICENSE-APACHE).
The encoding specifications ([`specs/`](./specs)) are
licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
