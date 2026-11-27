module

public import BijouAeneas.Refinement.U64
public import Bijou.SignedInstances

@[expose] public section

/-!
# `bijoux::i64` refines the bijou64s SPEC model

The signed format is `zigzag ∘ bijou64`, and so is the Rust: each `i64`
function maps through `zigzag` / `unzigzag` and calls the `u64` one. So the
proofs here are `zigzag_spec` / `unzigzag_spec` (the only new arithmetic)
composed with the `u64` refinement theorems.

- `zigzag_spec` / `unzigzag_spec` — the Rust bit tricks compute the model's
  `Bijou.zigzag` / `Bijou.unzigzag`.
- `encoded_len_spec` — `encoded_len v = encodedLenS v` for every `i64`.
- `encode_spec` — `encode v buf` appends exactly the model's `encodeS v`.
- `encoded_bytes_as_slice` — `encoded_bytes(v).as_slice()` is exactly the
  model's `encodeS v`.
- `decode_spec` — on *every* input, `decode` returns exactly the model's
  `decodeS` (same value and length, or the same error) and never panics.
-/

open Aeneas Aeneas.Std Result
open BijouRust

namespace BijouAeneas.I64

/-! ## `zigzag` and `unzigzag` match the model's maps

Rust's branchless `zigzag(n) = (n << 1) ^ (n >> 63)`: the arithmetic shift
smears the sign bit, so the XOR is a no-op for `n ≥ 0` and a bitwise NOT for
`n < 0` (`zz_pos` / `zz_neg`); the rest is arithmetic against
`Bijou.zigzag : ℤ → ℕ`. (Proven bit by bit rather than with `bv_decide`, whose
proofs trust compiled code via `Lean.ofReduceBool`: everything here depends
only on Lean's standard axioms.) -/

/-- The arithmetic shift by `63` smears the sign bit across the word. -/
theorem sshiftRight_sign (x : BitVec 64) :
    x.sshiftRight 63 = if x.msb then BitVec.allOnes 64 else 0#64 := by
  have hm : x.msb = x[63] := by
    rw [BitVec.msb_eq_getLsbD_last, BitVec.getLsbD_eq_getElem]
  ext i hi
  simp only [BitVec.getElem_sshiftRight]
  split
  · have : i = 0 := by omega
    subst this
    cases h : x.msb <;> simp only [h, Bool.false_eq_true, ite_false, ite_true,
      BitVec.getElem_allOnes, BitVec.getElem_zero] at hm ⊢ <;> simp [← hm]
  · cases h : x.msb <;> simp only [Bool.false_eq_true, ite_false, ite_true,
      BitVec.getElem_allOnes, BitVec.getElem_zero]

theorem zz_pos (x : BitVec 64) (h : x.msb = false) :
    (x <<< 1) ^^^ x.sshiftRight 63 = x <<< 1 := by
  rw [sshiftRight_sign, h]; exact BitVec.xor_zero

theorem zz_neg (x : BitVec 64) (h : x.msb = true) :
    (x <<< 1) ^^^ x.sshiftRight 63 = ~~~(x <<< 1) := by
  rw [sshiftRight_sign, h]; exact BitVec.xor_allOnes

theorem zigzag_bv (x : BitVec 64) :
    ((x <<< 1) ^^^ x.sshiftRight 63).toNat = Bijou.zigzag x.toInt := by
  have hlt := x.isLt
  cases hm : x.msb
  · rw [zz_pos x hm, BitVec.toNat_shiftLeft, BitVec.toInt_eq_toNat_of_msb hm]
    have h2 := (BitVec.msb_eq_false_iff_two_mul_lt).mp hm
    simp only [Bijou.zigzag, Nat.shiftLeft_eq]
    omega
  · rw [zz_neg x hm, BitVec.toNat_not, BitVec.toNat_shiftLeft]
    have h2 : 2 ^ 64 ≤ 2 * x.toNat := by
      have := (BitVec.msb_eq_false_iff_two_mul_lt (x := x)).not.mp (by simp [hm])
      omega
    have hti : x.toInt = Int.negSucc (2 ^ 64 - 1 - x.toNat) := by
      rw [BitVec.toInt_eq_msb_cond, hm, ite_eq_left (by rfl), Int.negSucc_eq]
      omega
    rw [hti]
    simp only [Bijou.zigzag, Nat.shiftLeft_eq]
    omega

@[step]
theorem zigzag_spec (n : Std.I64) : i64.zigzag n ⦃ z => z.val = Bijou.zigzag n.val ⦄ := by
  unfold i64.zigzag
  step*
  have hs : (IScalar.hcast UScalarTy.U64 i2).bv = i2.bv := by
    rw [IScalar.hcast_bv_eq]; exact BitVec.signExtend_eq _
  change (IScalar.hcast UScalarTy.U64 i2).bv.toNat = Bijou.zigzag n.bv.toInt
  rw [hs]
  simp only [i2_post1, i_post1, i1_post1]
  exact zigzag_bv n.bv

@[step]
theorem unzigzag_spec (u : Std.U64) : i64.unzigzag u ⦃ r => r.val = Bijou.unzigzag u.val ⦄ := by
  unfold i64.unzigzag
  have hu : u.val < 2 ^ 64 := by simpa using u.hBounds
  step*
  all_goals
    have h2 : i2.val = u.val % 2 := by
      rw [i2_post]; simp [UScalar.val_and, Nat.and_one_is_mod]
    have h3 : i3.val = ((u.val % 2 : ℕ) : ℤ) := by
      rw [i3_post, UScalar.hcast_val_eq, h2]
      apply Int.bmod_eq_of_le <;> simp <;> omega
  · rw [h3]; scalar_tac
  · have h1 : i1.val = ((u.val / 2 : ℕ) : ℤ) := by
      rw [i1_post, UScalar.hcast_val_eq, i_post, Nat.shiftRight_eq_div_pow]
      apply Int.bmod_eq_of_le <;> simp <;> omega
    have hbv : ∀ x : Std.I64, x.bv = BitVec.ofInt 64 x.val := fun x =>
      BitVec.ofInt_toInt.symm
    change (i1.bv ^^^ i4.bv).toInt = _
    rcases Nat.mod_two_eq_zero_or_one u.val with he | ho
    · have h4 : i4.bv = 0#64 := by rw [hbv i4, i4_post, h3, he]; rfl
      rw [h4, BitVec.xor_zero, Bijou.unzigzag, ite_eq_left he]
      change i1.val = _
      rw [h1]; rfl
    · have h4 : i4.bv = BitVec.allOnes 64 := by rw [hbv i4, i4_post, h3, ho]; rfl
      have hl : i1.bv.toNat < 2 ^ 64 := by simpa using i1.bv.isLt
      have hn : (i1.bv.toNat : ℤ) = ((u.val / 2 : ℕ) : ℤ) := by
        have h := h1
        change i1.bv.toInt = _ at h
        rw [BitVec.toInt_eq_toNat_bmod] at h
        simp only [Int.bmod] at h
        push_cast at h
        split at h <;> omega
      rw [h4, BitVec.xor_allOnes, BitVec.toInt_not, Bijou.unzigzag, ite_eq_right (by omega),
        Int.negSucc_eq]
      simp only [Int.bmod]
      push_cast
      split <;> omega

/-! ## The signed functions refine the model's signed family -/

theorem encoded_len_spec (v : Std.I64) :
    i64.encoded_len v ⦃ n => n.val = Bijou.bijou64.encodedLenS v.val ⦄ := by
  unfold i64.encoded_len
  apply WP.spec_bind (zigzag_spec v)
  intro z hz
  apply WP.spec_mono (BijouAeneas.U64.encoded_len_spec z)
  intro n hn
  rw [hn, hz]
  rfl

theorem encode_spec (v : Std.I64) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 9 ≤ Usize.max) :
    i64.encode v buf ⦃ out =>
      out.val.map (·.val) = buf.val.map (·.val) ++ Bijou.bijou64.encodeS v.val ⦄ := by
  unfold i64.encode
  apply WP.spec_bind (zigzag_spec v)
  intro z hz
  apply WP.spec_mono (BijouAeneas.U64.encode_spec z buf hbuf)
  intro out hout
  rw [hout, hz]
  rfl

/-- What callers see: `encoded_bytes(v).as_slice()` is exactly the model's
signed encoding of `v`. -/
theorem encoded_bytes_as_slice (v : Std.I64) :
    i64.encoded_bytes v ⦃ e => i64.EncodedI64.as_slice e ⦃ s =>
      s.val.map (·.val) = Bijou.bijou64.encodeS v.val ⦄ ⦄ := by
  unfold i64.encoded_bytes i64.EncodedI64.as_slice
  apply WP.spec_bind (zigzag_spec v)
  intro z hz
  have hs : Bijou.bijou64.encodeS v.val = Bijou.bijou64.encode z.val := by
    rw [hz]; rfl
  rw [hs]
  apply WP.spec_bind (BijouAeneas.U64.encoded_bytes_as_slice z)
  intro e he
  simpa using he

/-- The Rust signed decode result, read as the model's. -/
def toModelS : core.result.Result (Std.I64 × Std.Usize) u64.DecodeError →
    Except Bijou.DecodeError (ℤ × ℕ)
  | .Ok (v, n) => .ok (v.val, n.val)
  | .Err .BufferTooShort => .error .bufferTooShort
  | .Err .Overflow => .error .overflow

theorem decode_spec (buf : Slice Std.U8) :
    i64.decode buf ⦃ r => toModelS r = Bijou.bijou64.decodeS (buf.val.map (·.val)) ⦄ := by
  unfold i64.decode
  apply WP.spec_bind (BijouAeneas.U64.decode_spec buf)
  intro r hr
  match r with
  | .Ok (value, consumed) =>
    simp only
    apply WP.spec_bind (unzigzag_spec value)
    intro i hi
    simp only [WP.spec_ok, toModelS, Bijou.Family.decodeS, ← hr, BijouAeneas.U64.toModel, hi]
  | .Err .BufferTooShort =>
    simp only [WP.spec_ok, toModelS, Bijou.Family.decodeS, ← hr, BijouAeneas.U64.toModel]
  | .Err .Overflow =>
    simp only [WP.spec_ok, toModelS, Bijou.Family.decodeS, ← hr, BijouAeneas.U64.toModel]

end BijouAeneas.I64
