# Formal verification

The bijou format and its Rust implementation are verified in Lean 4, in two layers:

```
 specs/*.md ──(transcribed)──▶ lean/          the model: Bijou.Family
                                 ▲            format theorems (round-trip,
                                 │            canonicality, order, …)
                          refines│
                                 │
 bijoux/src ──Charon──▶ LLBC ──Aeneas──▶ lean-aeneas/   the real Rust,
                                         Generated/     as pure Lean
                                         Refinement/    proofs that it
                                                        equals the model
```

- **`lean/`** proves the _format_ is sound. It is a model of the bijou family, parametrized over the tier count, so one development covers `bijou32`, `bijou64`, and `bijou128`.
- **`lean-aeneas/`** proves the _code_ implements the format. [Charon] and [Aeneas] translate `bijoux` into Lean. The refinement proofs show that each translated function returns exactly what the model prescribes. The model's theorems then carry over to the Rust functions.

## What is proven

### The format (`lean/`)

| Theorem                                                      | Statement                                                                                                   |
|--------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| `decode_encode`                                              | Decoding an encoding returns the original value, with or without trailing bytes                             |
| `decode_ok`                                                  | Any byte string the decoder accepts is exactly `encode` of the returned value (no overlong encodings exist) |
| `encode_injective` / `decode_canonical` / `encode_bijection` | `encode` is a bijection between `[0, 256 ^ tiers)` and the accepted byte strings                            |
| `encode_lex_iff` / `encode_lex_trichotomy`                   | Lexicographic byte order equals numeric order                                                               |
| `decode_consumed_from_tag`                                   | The encoding length is a function of the first byte alone                                                   |
| `encodedLen_eq_one_iff` / `encodedLen_eq_succ_iff` / `encodedLen_mono` | The tier table in general: one byte exactly below the threshold, `t + 1` bytes exactly on `[offset t, offset (t+1))`, and larger values never encode shorter |
| `encodedLenS_eq_one_iff` / `encodedLenS_eq_succ_iff` / `encodedLenS_mono` | Signed: one byte exactly on `[-⌊T/2⌋, ⌈T/2⌉)` (`-124 ..= 123` for bijou64s), the signed tier table, and a larger magnitude never encodes shorter, whatever the signs |
| `decodeS_encodeS`, `encodeS_lex_iff`, …                      | The same properties for the signed (zigzag) family                                                          |

### The Rust code (`lean-aeneas/`)

All six modules, `bijoux::{u32, u64, u128}` and `bijoux::{i32, i64, i128}`, have the same four theorems. The unsigned ones are against `Bijou.bijou32`, `bijou64` and `bijou128` (`encodedLen`, `encode`, `decode`). The signed ones are against the same families' signed operations (`encodedLenS`, `encodeS`, `decodeS`):

| Function      | Spec theorem       | Guarantee                                                                                                                                                   |
|---------------|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `encoded_len` | `encoded_len_spec` | For every value: never panics, returns the model's `encodedLen`                                                                                             |
| `encode`      | `encode_spec`      | For every value: appends exactly the model's `encode` bytes (see the preconditions below)                                                                   |
| `decode`      | `decode_spec`      | For _every_ input slice, including malformed ones: never panics, and returns exactly the model's `decode`, with the same value and length or the same error |
| `encoded_bytes` + `as_slice` | `encoded_bytes_as_slice` | For every value: `encoded_bytes(v).as_slice()` is exactly the model's encoding of `v`, the same bytes `encode` appends, without allocating |

The signed functions are `zigzag`/`unzigzag` wrapped around the unsigned ones, in the Rust as in the model. So their proofs are `zigzag_spec` and `unzigzag_spec`, which show that Rust's branchless bit tricks compute the model's `zigzag : ℤ → ℕ` and its inverse, composed with the unsigned specs.

The specs are full equalities with the model. The corollaries in `Refinement/*Corollaries.lean` restate the model's theorems directly about the Rust functions:

| Corollary                  | Statement about the Rust                                                                       |
|----------------------------|------------------------------------------------------------------------------------------------|
| `decode_encoded`           | `decode` on bytes that start with `encode v` returns `Ok (v, encoded_len v)`, whatever follows |
| `decode_after_encode`      | `encode v` into an empty buffer, then `decode`, gives `Ok (v, …)`                              |
| `decode_canonical`         | If `decode` returns `Ok (v, n)`, the first `n` input bytes are exactly the encoding of `v`     |
| `decode_consumed_from_tag` | On success, the number of bytes consumed depends only on the first byte                        |
| `encode_injective`         | Different values never produce the same bytes                                                  |
| `encode_lex_iff`           | Unsigned: the appended bytes compare lexicographically exactly as the values compare numerically. Signed: they compare in zigzag order (`0, -1, 1, -2, 2, …`), not numeric order |
| `encoded_len_eq_appended`  | `encoded_len v` is the number of bytes `encode v` appends                                      |

## What is trusted

A proof is only as strong as what it assumes. These are the assumptions that the proofs do not check:

1. **The model is the spec.** `lean/` is a transcription of `specs/`. The format theorems show that the model is internally sound, but agreement between the model and the prose spec is checked by review.
2. **The translation is faithful.** The proofs are about the Lean code that Charon and Aeneas produce, not about the machine code that `rustc` produces. Errors in Charon, Aeneas, or `rustc` are outside the proofs. The versions are pinned in `flake.nix`.
3. **The `core`/`alloc` models are correct.** Aeneas does not translate the standard library. `Generated/FunsExternal.lean` defines by hand the functions that `bijoux` calls:

   | Rust function                  | Model                                                              |
   |--------------------------------|--------------------------------------------------------------------|
   | `u32/u64/u128::saturating_mul` | the product, clamped to `MAX`                                      |
   | `<[T]>::split_first`           | `None` on an empty slice, otherwise the head and the rest          |
   | `<[T]>::first_chunk::<N>`      | the first `N` elements if there are at least `N`, otherwise `None` |
   | `Vec::truncate`                | keep the first `n` elements                                        |

   These are plain definitions, not axioms, so each is a few lines that you can compare with the `core`/`alloc` documentation. `Refinement/FunsExternalChecks.lean` also proves each one equal to the function as `core`/`alloc` implements it, written with operations Aeneas already models (`checked_mul`, `index_usize`, `split_at`). A typo in a model breaks those proofs, so what remains trusted is `Aeneas.Std` itself. All other library functions (`leading_zeros`, `to_be_bytes`, `from_be_bytes`, `Vec::extend_from_slice`, …) come from Aeneas's own library, `Aeneas.Std`.
4. **The Lean kernel and its dependencies.** These are Lean 4.36, Mathlib, and the Aeneas Lean library, pinned in `lake-manifest.json`. Every theorem depends only on Lean's three standard axioms (`propext`, `Classical.choice`, `Quot.sound`). No proof uses `sorry`, `native_decide` or `bv_decide`, which would also trust compiled code. CI enforces this: `AxiomAudit.lean` fails if any theorem in `Bijou.*` or `BijouAeneas.*` depends on another axiom. The SPEC test vectors and tables are kernel-checked too (`decide +kernel` in `lean/Bijou/Instances.lean` and `SignedInstances.lean`), not run as compiled code, and each is a named theorem so the audit covers them.

### Preconditions

`encode_spec` requires `buf.len() + max_bytes ≤ usize::MAX`, where `max_bytes` is 5, 9 or 17 for the 32-, 64- or 128-bit widths. For a larger buffer, the Rust `Vec` panics with a capacity overflow before `encode` can append. The other specs have no preconditions.

## Layout

```
lean-aeneas/
├── BijouAeneas/
│   ├── Generated/              # from `aeneas:translate`; do not edit
│   │   ├── Funs.lean           #   the translated functions
│   │   ├── Types.lean          #   the translated types
│   │   ├── FunsExternal_Template.lean
│   │   └── FunsExternal.lean   # hand-written core/alloc models (trusted)
│   └── Refinement/
│       ├── Bytes.lean          # Aeneas byte conversions vs the model's beBytes/fromBe
│       ├── CoreAlloc.lean      # step specs for the core/alloc models
│       ├── U64.lean            # encoded_len_spec, encode_spec, decode_spec
│       ├── U64Corollaries.lean # the format properties, restated for the Rust
│       ├── I64.lean            # zigzag_spec, unzigzag_spec, and the signed specs
│       ├── I64Corollaries.lean # the signed format properties, restated for the Rust
│       └── {U,I}{32,128}*.lean # generated from the 64-bit files (see below)
├── scripts/gen_widths.py       # derives the 32/128-bit proofs from the 64-bit ones
├── lakefile.toml               # requires Aeneas's Lean library and ../lean
└── lean-toolchain
```

## Running the proofs

Use the proofs dev shell. It adds Charon and Aeneas to the default shell:

```sh
nix develop .#proofs
proofs              # check the model in lean/
proofs:aeneas       # re-translate, check Generated/ is up to date, check lean-aeneas/
aeneas:translate    # regenerate Generated/ after changing bijoux/src
```

Or without entering a shell:

```sh
nix run .#ci-lean       # the model
nix run .#ci-aeneas     # the refinement proofs
```

The first `ci-aeneas` run downloads a prebuilt Mathlib (about 7 GB).

> [!IMPORTANT]
> The proofs are about the _committed_ translation. If you change the translated code in `bijoux/src/{u32,u64,u128,i32,i64,i128}.rs`, run `aeneas:translate` and commit the new `Generated/` files. CI fails if they are out of date. A change can make a refinement proof fail. If the change is intended, update the proof. If not, the proof found a behavior change.

### Editing the proofs

The 32-, 64- and 128-bit Rust code is the same apart from its constants, so the proofs are too. The four 64-bit files (`U64`, `U64Corollaries`, `I64`, `I64Corollaries`) are written by hand, except for `decode_spec`: its shape depends on the tier count (one `match` arm per tier), so every width's `decode_spec`, `U64.lean`'s included, is generated from a template in `scripts/gen_widths.py`. The 32- and 128-bit files are generated from the 64-bit ones by the same script, which substitutes the width constants and Aeneas's local variable names. CI re-runs it and fails if the committed files differ. To change a proof, edit the 64-bit file (or, for `decode_spec`, the template), then regenerate:

```sh
python3 lean-aeneas/scripts/gen_widths.py lean-aeneas/BijouAeneas/Refinement
```

[Aeneas]: https://github.com/AeneasVerif/aeneas
[Charon]: https://github.com/AeneasVerif/charon
