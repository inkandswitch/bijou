module

public import BijouAeneas.Generated.Funs

@[expose] public section

/-!
# Cross-checks for the hand-written `core`/`alloc` models

`Generated/FunsExternal.lean` defines, by hand, the four `core`/`alloc`
functions that Aeneas leaves opaque. Those definitions are trusted: if one
said something other than what Rust's function does, the refinement proofs
would describe code that doesn't behave like the real library.

Each theorem here restates one model the way `core`/`alloc` itself implements
the function, using only operations Aeneas already models in `Aeneas.Std`
(`checked_mul`, `index_usize`, `split_at`), and proves the two agree. A typo
in a model now breaks a proof; what remains trusted is `Aeneas.Std`, as for
every other library call (`leading_zeros`, `to_be_bytes`, …).

| Model | `core`/`alloc` implementation | Checked against |
|---|---|---|
| `saturating_mul` | `checked_mul(rhs).unwrap_or(MAX)` | `checked_mul` |
| `split_first` | `if let [first, tail @ ..] = self` | `index_usize 0`, `split_at 1` |
| `first_chunk::<N>` | `None` if `len < N`, else the first `N` | `split_at N` |
| `Vec::truncate` | no-op if `len ≥ self.len`, else the first `len` | `split_at len` |
-/

open Aeneas Aeneas.Std Result
open BijouRust

namespace BijouAeneas

/-- Clamping the product at `max` is `checked_mul`, or `max` on overflow --
`core`'s definition of `saturating_mul`, for every unsigned width. -/
theorem clamp_is_checked_or_max {ty : UScalarTy} (x y : UScalar ty) :
    (⟨BitVec.ofNat _ (Min.min (UScalar.max ty) (x.val * y.val))⟩ : UScalar ty) =
      (core.num.checked_mul_UScalar x y).getD ⟨BitVec.ofNat _ (UScalar.max ty)⟩ := by
  have h := core.num.checked_mul_UScalar_bv_spec x y
  have hmax : UScalar.max ty < 2 ^ ty.numBits := by
    simp only [UScalar.max]; have := Nat.one_le_two_pow (n := ty.numBits); omega
  split at h <;> rename_i heq <;> rw [heq] <;> simp only [Option.getD_some, Option.getD_none]
  · apply UScalar.eq_of_val_eq
    simp only [UScalar.val, BitVec.toNat_ofNat] at h ⊢
    rw [Nat.min_eq_right h.1, h.2.1]
    exact Nat.mod_eq_of_lt (by omega)
  · rw [Nat.min_eq_left (Nat.le_of_lt h)]

theorem saturating_mul_u32_agrees (x y : Std.U32) :
    core.num.U32.saturating_mul x y =
      ok ((core.num.checked_mul_UScalar x y).getD core.num.U32.MAX) := by
  have hm : (⟨BitVec.ofNat _ (UScalar.max .U32)⟩ : UScalar .U32) = core.num.U32.MAX := by
    apply UScalar.eq_of_val_eq
    simp only [UScalar.val, core.num.U32.MAX, UScalar.ofNat, UScalar.ofNatCore, BitVec.toNat_ofNat,
      UScalar.max_UScalarTy_U32_eq, U32.max_eq, U32.rMax]
    decide
  rw [core.num.U32.saturating_mul, clamp_is_checked_or_max, hm]

/-- `u64::saturating_mul` as `core` writes it:
`match self.checked_mul(rhs) { Some(x) => x, None => Self::MAX }`,
i.e. `checked_mul(rhs).unwrap_or(MAX)`. -/
theorem saturating_mul_u64_agrees (x y : Std.U64) :
    core.num.U64.saturating_mul x y =
      ok ((core.num.checked_mul_UScalar x y).getD core.num.U64.MAX) := by
  have hm : (⟨BitVec.ofNat _ (UScalar.max .U64)⟩ : UScalar .U64) = core.num.U64.MAX := by
    apply UScalar.eq_of_val_eq
    simp only [UScalar.val, core.num.U64.MAX, UScalar.ofNat, UScalar.ofNatCore, BitVec.toNat_ofNat,
      UScalar.max_UScalarTy_U64_eq, U64.max_eq, U64.rMax]
    decide
  rw [core.num.U64.saturating_mul, clamp_is_checked_or_max, hm]

theorem saturating_mul_u128_agrees (x y : Std.U128) :
    core.num.U128.saturating_mul x y =
      ok ((core.num.checked_mul_UScalar x y).getD core.num.U128.MAX) := by
  have hm : (⟨BitVec.ofNat _ (UScalar.max .U128)⟩ : UScalar .U128) = core.num.U128.MAX := by
    apply UScalar.eq_of_val_eq
    simp only [UScalar.val, core.num.U128.MAX, UScalar.ofNat, UScalar.ofNatCore, BitVec.toNat_ofNat,
      UScalar.max_UScalarTy_U128_eq, U128.max_eq, U128.rMax]
    decide
  rw [core.num.U128.saturating_mul, clamp_is_checked_or_max, hm]

/-- `<[T]>::split_first` as `core` writes it (`[first, tail @ ..]`): element `0`
and the slice from `1`, or `None` when empty. -/
theorem split_first_agrees {T : Type} [Inhabited T] (s : Slice T) :
    core.slice.Slice.split_first s =
      if s.length = 0 then ok none
      else (do
        let x ← Slice.index_usize s 0#usize
        let p ← core.slice.Slice.split_at s 1#usize
        ok (some (x, p.2))) := by
  simp only [core.slice.Slice.split_first]
  split
  · rename_i h
    simp [Slice.length, h]
  · rename_i x xs h
    have hl : s.length ≠ 0 := by simp [Slice.length, h]
    have h0 : (0 : ℕ) < s.length := by omega
    have hidx : Slice.index_usize s 0#usize = ok x := by
      simp [Slice.index_usize, h]
    have h1 : 1 ≤ s.length := by omega
    simp only [hl, ite_false, hidx, core.slice.Slice.split_at,
      show (1#usize : Std.Usize).val ≤ s.length by simpa using h1, _root_.dite_true]
    simp only [bind_ok]
    congr 3
    apply Slice.ext
    simp [Slice.from_val, h]

/-- `<[T]>::first_chunk::<N>` as `core` writes it: `None` if shorter than
`N`, else the first `N` elements (the `split_at N` head). -/
theorem first_chunk_agrees {T : Type} (N : Std.Usize) (s : Slice T) :
    core.slice.Slice.first_chunk N s ⦃ o => match o with
      | none => s.length < N.val
      | some a => core.slice.Slice.split_at s N ⦃ p => a.val = p.1.val ⦄ ⦄ := by
  simp only [core.slice.Slice.first_chunk, core.slice.Slice.split_at]
  split
  · rename_i h
    simp [WP.spec_ok, Array.make, Slice.length]
  · rename_i h
    simp only [WP.spec_ok]
    simp [Slice.length] at h ⊢; omega

/-- `Vec::truncate` as `alloc` writes it: a no-op when `len ≥ self.len`, else
the first `len` elements (the `split_at len` head). -/
theorem truncate_agrees {T : Type} (A : Type) (v : alloc.vec.Vec T) (n : Std.Usize) :
    alloc.vec.Vec.truncate A v n ⦃ w =>
      if v.length ≤ n.val then w = v
      else core.slice.Slice.split_at (alloc.vec.Vec.deref v) n ⦃ p => w.val = p.1.val ⦄ ⦄ := by
  simp only [alloc.vec.Vec.truncate, WP.spec_ok, core.slice.Slice.split_at, alloc.vec.Vec.deref]
  split
  · rename_i h
    apply alloc.vec.Vec.ext
    simp [List.take_of_length_le (by simpa using h)]
  · rename_i h
    have hn : n.val ≤ v.length := Nat.le_of_lt (Nat.lt_of_not_le h)
    simp [Slice.from_val, Slice.length, hn, WP.spec_ok, alloc.vec.Vec.from_val]

end BijouAeneas
