import BijouAeneas.Generated

/-!
# Refinement: generated Rust ⟶ format model

Proofs that the Aeneas-translated `bijou64` functions in
`BijouAeneas.Generated` satisfy their specifications, and ultimately
that they refine the parametrized format model in the `lean/` project
(`Bijou.Family`, instantiated at `bijou64`).

## Status

The pipeline (Charon → Aeneas → Lean) and this project's toolchain are
proven working: `BijouAeneas.Generated` typechecks against the real
Aeneas runtime library. The refinement proofs below are developed
incrementally; this module contains no `sorry`.

The loop-free base cases of `tier_offset` are proven here and matched to
the model's offset table (`Bijou.Family.offset` at `bijou64`: `offset 0
= 0`, `offset 1 = 248`). The remaining obligations are genuine work,
recorded in `../.ignore/TODO.md`; the two blockers are:

1. The generated `OFFSETS`/`BOUNDS` globals are built by a loop over
   `core.num.U64.saturating_mul`, which this Aeneas version emits as an
   un-specified `axiom`. Reasoning about them (even totality) first
   needs a trusted spec for that intrinsic.
2. `encode`/`encoded_len` tier dispatch goes through `leading_zeros`
   (modelled as `Nat.log 2`); relating `(bw-1)/8+2` to the tier is the
   crux lemma, a good `bv_decide`/`omega` target.
-/

namespace BijouAeneas

open Aeneas Aeneas.Std Result
open BijouRust

/-- The bijou64 offset recurrence, restated locally. The `lean/` model
(`Bijou.Family.offset` at `bijou64`) lives in a separate, Mathlib-free
project on a different Lean toolchain, so it can't be imported here; we
mirror it and keep both honest with the shared SPEC offset table, which
`#guard` checks below. -/
def modelOffset : Nat → Nat
  | 0 => 0
  | 1 => 248
  | (n + 2) => modelOffset (n + 1) + 256 ^ (n + 1)

-- `modelOffset` reproduces the SPEC.md "Offset Table" for bijou64.
#guard modelOffset 0 = 0
#guard modelOffset 1 = 0xF8
#guard modelOffset 2 = 0x1F8
#guard modelOffset 3 = 0x101F8
#guard modelOffset 8 = 0x1010101010101F8

/-- Tier 0 has offset 0. -/
@[simp]
theorem tier_offset_zero : tier_offset 0#usize = ok 0#u64 := by
  simp [tier_offset]

/-- Tier 1 has offset 248 (the tag threshold) — the first value that no
longer fits in a single byte. -/
@[simp]
theorem tier_offset_one : tier_offset 1#usize = ok 248#u64 := by
  simp only [tier_offset, TAG_THRESHOLD]
  rfl

/-- The generated `tier_offset` agrees with the local `modelOffset` at
tier 0 — an in-project, machine-checked correspondence between the
translated Rust and the offset model. -/
theorem tier_offset_zero_matches_model :
    ∃ x : Std.U64, tier_offset 0#usize = ok x ∧ x.val = modelOffset 0 :=
  ⟨0#u64, tier_offset_zero, by decide⟩

/-- The generated `tier_offset` agrees with `modelOffset` at tier 1. -/
theorem tier_offset_one_matches_model :
    ∃ x : Std.U64, tier_offset 1#usize = ok x ∧ x.val = modelOffset 1 :=
  ⟨248#u64, tier_offset_one, by decide⟩

end BijouAeneas
