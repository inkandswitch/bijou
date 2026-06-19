import BijouAeneas.Generated
import BijouAeneas.Refinement

/-!
# BijouAeneas

Phase 2 of the bijou formal-verification effort: proofs about the
*actual Rust* in `bijou64/src/lib.rs`, as translated to Lean by Charon +
Aeneas.

- `BijouAeneas.Generated` — the Aeneas output (auto-generated; do not
  edit). Covers `encode`, `encoded_len`, and the offset machinery. See
  its header for the exact pipeline and tool versions.
- `BijouAeneas.Refinement` — proofs that the generated functions satisfy
  their specifications and refine the format model from the (separate)
  `lean/` project.

`decode` is verified with Kani instead of Aeneas (its slice
rest-patterns are unsupported by this Aeneas version); see
`../.ignore/aeneas/SPIKE.md`.
-/
