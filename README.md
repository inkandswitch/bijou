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
ci                   # fmt + clippy + test + no_std + wasm32 + Lean proofs
bench:shootout       # criterion shootout vs other varints
bench:gungraun       # gungraun instruction-count benchmarks
proofs               # check the Lean 4 proofs in lean/
```

Without Nix:

```sh
cargo build --workspace
cargo test --workspace --all-features
```

The workspace targets stable Rust (see `rust-version` in
`Cargo.toml`) and supports `wasm32-unknown-unknown` via the
toolchain shipped in the flake.

## Formal proofs

The [`lean/`](./lean) directory contains a Lean 4 model of the format,
parametrized over the tier count so one development covers all three
width variants. Machine-checked theorems include:

- _Round-trip_: decoding an encoding returns the original value.
- _Canonicality by construction_: any byte string the decoder accepts
  is exactly the canonical encoding of the value it returns — there is
  no overlong encoding to reject.
- _Bijectivity_: `encode` is injective, and fully-consumed buffers
  decoding to the same value are identical.
- _Order preservation_: lexicographic byte order equals numeric order
  (a strict total order on encodings).
- _Framing_: encoding length is determined by the first byte alone
  (O(1) skipping), never exceeds `maxBytes` (9/5/17), and overflow can
  occur only at the top tier (tag `0xFF`).

Every test vector in the three `SPEC.md` documents is also checked by
reduction. Run the proofs with `proofs` inside the dev shell, or
hermetically via `nix build .#checks.<system>.lean-proofs`.

This model describes the specified _format_. Connecting it to the
_actual Rust_ is a second, in-progress layer:

- **Aeneas** ([`lean-aeneas/`](./lean-aeneas)) translates `bijou64`'s
  Rust to Lean via Charon and proves it refines the model above.
  `encode` and `encoded_len` translate cleanly; run with `proofs:aeneas`
  (needs network for Mathlib + Aeneas on first build).
- **Kani** (`bijou64/src/kani_proofs.rs`) bounded-model-checks `decode`,
  whose slice patterns are outside Aeneas's current support. It verifies
  totality, canonicality (no overlong encodings), round-trip, and the
  error conditions. Run with `proofs:kani` (needs upstream Kani, which
  is not packaged for NixOS).

Both Phase-2 layers run outside the hermetic Nix `ci` (heavy toolchains,
network, non-NixOS Kani). Until they fully land, "the Rust matches the
model" also rests on shared SPEC vectors and `bolero` property tests.

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
