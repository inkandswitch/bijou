module

public import BijouAeneas.Generated.Funs
public import BijouAeneas.Refinement.Bytes
public import BijouAeneas.Refinement.CoreAlloc
public import Bijou.Instances
public import Mathlib.Data.Nat.Log
public import Mathlib.Tactic.IntervalCases
public import Mathlib.Tactic.NormNum
-- `sat_add_val` unfolds Aeneas's `saturating_add`, whose body that module
-- does not expose (and for which Aeneas has no spec lemma).
import all Aeneas.Std.Scalar.SaturatingOps

@[expose] public section

/-!
# `bijoux::u32` refines the bijou32 SPEC model

Each theorem states that a function translated from the real Rust
(`BijouAeneas.Generated`) never panics and returns exactly what the
corresponding `Bijou.Family` definition prescribes for `Bijou.bijou32`.

The proofs are `U64.lean`'s, with the width constants and Aeneas's
variable names substituted (`scripts/gen_widths.py`). Change `U64.lean`
first, then regenerate this file rather than editing it by hand.

- `tier_offset_spec` — the `tier_offset` loop computes the model's tier
  offsets; its saturating arithmetic never saturates for tiers `≤ 4`.
- `BOUNDS_spec` — so the `BOUNDS` table holds `offset 1 .. offset 4`.
- `tier_dispatch` — the `leading_zeros` tier dispatch (`(bw - 1) / 8 + 2`,
  corrected by one `BOUNDS` comparison) always lands on the model's tier.
- `encoded_len_spec` — `encoded_len v = encodedLen v` for every `u32`.
- `encode_spec` — `encode v buf` appends exactly the model's `encode v`.
- `encoded_bytes_as_slice` — `encoded_bytes(v).as_slice()` is exactly the
  model's `encode v` (via `encoded_bytes_spec` and `as_slice_spec`).
- `decode_spec` — on *every* input, `decode` returns exactly the model's
  `decode` (same value and length, or the same error) and never panics.
-/

open Aeneas Aeneas.Std Result
open BijouRust

namespace BijouAeneas.U32

theorem sat_mul_val (x y : Std.U32) (h : x.val * y.val ≤ U32.max) :
    core.num.U32.saturating_mul x y ⦃ z => z.val = x.val * y.val ⦄ := by
  have hW : (2 : ℕ) ^ UScalarTy.U32.numBits = 4294967296 := rfl
  rw [U32.max_eq] at h
  simp only [core.num.U32.saturating_mul, WP.spec_ok, UScalar.max_UScalarTy_U32_eq, U32.max_eq,
    UScalar.val, BitVec.toNat_ofNat, hW] at h ⊢
  omega

theorem sat_add_val (x y : Std.U32) (h : x.val + y.val ≤ U32.max) :
    (core.num.U32.saturating_add x y).val = x.val + y.val := by
  have hW : (2 : ℕ) ^ UScalarTy.U32.numBits = 4294967296 := rfl
  rw [U32.max_eq] at h
  simp only [core.num.U32.saturating_add, UScalar.saturating_add,
    UScalar.max_UScalarTy_U32_eq, U32.max_eq,
    UScalar.val, BitVec.toNat_ofNat, hW] at h ⊢
  omega

theorem tier_offset_loop_spec (n : Std.Usize) (hn : n.val ≤ 4)
    (result power : Std.U32) (i : Std.Usize)
    (hi : 2 ≤ i.val) (hin : i.val ≤ n.val + 1)
    (hr : result.val = Bijou.bijou32.offset (i.val - 1))
    (hp : power.val = 256 ^ (i.val - 2)) :
    u32.tier_offset_loop n result power i ⦃ r => r.val = Bijou.bijou32.offset n.val ⦄ := by
  unfold u32.tier_offset_loop
  apply loop.spec_decr_nat (fun (x : Std.U32 × Std.U32 × Std.Usize) => n.val + 1 - x.2.2.val)
    (fun x => 2 ≤ x.2.2.val ∧ x.2.2.val ≤ n.val + 1 ∧
      x.1.val = Bijou.bijou32.offset (x.2.2.val - 1) ∧ x.2.1.val = 256 ^ (x.2.2.val - 2))
  · rintro ⟨r, p, j⟩ ⟨hj2, hjn, hr, hp⟩
    simp only [u32.tier_offset_loop.body]
    split
    · rename_i hle
      simp only at hj2 hjn hr hp
      have hjn' : j.val ≤ n.val := by scalar_tac
      have hp256 : p.val * 256 = 256 ^ (j.val - 1) := by
        rw [hp, ← pow_succ]
        congr 1
        omega
      have hplt : 256 ^ (j.val - 1) ≤ U32.max := by
        rw [U32.max_eq]
        calc 256 ^ (j.val - 1) ≤ 256 ^ 3 := Nat.pow_le_pow_right (by norm_num) (by omega)
          _ ≤ 4294967295 := by norm_num
      have hoff : Bijou.bijou32.offset (j.val - 1) + 256 ^ (j.val - 1) =
          Bijou.bijou32.offset j.val := by
        have h := Bijou.bijou32.offset_succ (j.val - 1)
        rw [Bijou.bijou32.capacity_eq_pow (by omega), show j.val - 1 + 1 = j.val by omega] at h
        omega
      have hofflt : Bijou.bijou32.offset j.val ≤ U32.max := by
        have h := Bijou.bijou32.offset_lt_pow (t := j.val) (by omega)
        have h8 : (256 : ℕ) ^ j.val ≤ 256 ^ 4 := Nat.pow_le_pow_right (by norm_num) (by omega)
        rw [U32.max_eq]
        have : (256 : ℕ) ^ 4 = 4294967296 := by norm_num
        omega
      step with sat_mul_val p 256#u32 (by simp; omega)
      simp only [lift]
      step*
      have hsum : r.val + power1.val ≤ U32.max := by
        rw [hr, power1_post, hp256, hoff]; exact hofflt
      rw [sat_add_val r power1 hsum, hr, power1_post, hp256, hoff,
        show i1.val - 1 = j.val by omega, show i1.val - 2 = j.val - 1 by omega]
      omega
    · rename_i hgt
      simp only at hj2 hjn hr hp
      have hj : j.val = n.val + 1 := by scalar_tac
      simp only [WP.spec_ok, hr, hj, Nat.add_sub_cancel]
  · exact ⟨hi, hin, hr, hp⟩

theorem tag_threshold_cast :
    (UScalar.cast UScalarTy.U32 u32.TAG_THRESHOLD).val = 252 := by
  unfold u32.TAG_THRESHOLD
  simp

@[step]
theorem tier_offset_spec (n : Std.Usize) (h : n.val ≤ 4) :
    u32.tier_offset n ⦃ r => r.val = Bijou.bijou32.offset n.val ⦄ := by
  unfold u32.tier_offset
  split
  · rename_i h0
    have : n.val = 0 := by scalar_tac
    simp [WP.spec_ok, this]
  · split
    · rename_i h0 h1
      have : n.val = 1 := by scalar_tac
      simp only [WP.spec_ok, tag_threshold_cast, this, Bijou.Family.offset_one]
      rfl
    · rename_i h0 h1
      have h2 : 2 ≤ n.val := by scalar_tac
      simp only [lift]
      step with tier_offset_loop_spec n h _ 1#u32 2#usize (by simp) (by simp; omega)
        (by rw [tag_threshold_cast]; rfl) (by simp)
      assumption

@[step]
theorem BOUNDS_spec :
    u32.BOUNDS ⦃ a => ∀ i < 4, (a.val[i]!).val = Bijou.bijou32.offset (i + 1) ⦄ := by
  unfold u32.BOUNDS
  step*
  intro j hj
  interval_cases j <;> simp [*]

/-- The Rust tier dispatch, as a statement about the model: with
`L = ⌊log₂ v⌋` (so the byte length is `L / 8 + 1`), the encoded length is
that byte length if `v` is below the next tier's offset, else one more. -/
theorem tier_dispatch {v : ℕ} (hlo : 252 ≤ v) (hhi : v < 2 ^ 32) :
    Bijou.bijou32.encodedLen v =
      if v < Bijou.bijou32.offset (Nat.log 2 v / 8 + 1) then Nat.log 2 v / 8 + 1
      else Nat.log 2 v / 8 + 2 := by
  have hthr : Bijou.bijou32.threshold = 252 := rfl
  have htiers : (256 : ℕ) ^ Bijou.bijou32.tiers = 2 ^ 32 := by norm_num [Bijou.bijou32]
  have hnlt : ¬ v < Bijou.bijou32.threshold := by omega
  simp only [Bijou.Family.encodedLen, hnlt, ite_false]
  set L := Nat.log 2 v with hL
  have h1 : 2 ^ L ≤ v := Nat.pow_log_le_self 2 (by omega)
  have h2 : v < 2 ^ (L + 1) := Nat.lt_pow_succ_log_self (by norm_num) v
  have hv : v < 256 ^ Bijou.bijou32.tiers := by rw [htiers]; exact hhi
  split
  · rename_i hlt
    have hlow : Bijou.bijou32.offset (L / 8) ≤ v := by
      rcases Nat.eq_zero_or_pos (L / 8) with h0 | hpos
      · rw [h0]; simp
      · have hoff := Bijou.bijou32.offset_lt_pow hpos
        have hpow : 256 ^ (L / 8) ≤ 2 ^ L := by
          rw [show (256 : ℕ) = 2 ^ 8 by norm_num, ← pow_mul]
          exact Nat.pow_le_pow_right (by norm_num) (by omega)
        omega
    have := Bijou.bijou32.tierOf_eq hv hlow hlt
    omega
  · rename_i hge
    have hpow : 2 ^ (L + 1) ≤ 256 ^ (L / 8 + 1) := by
      rw [show (256 : ℕ) = 2 ^ 8 by norm_num, ← pow_mul]
      exact Nat.pow_le_pow_right (by norm_num) (by omega)
    have hcap : Bijou.bijou32.capacity (L / 8 + 1) = 256 ^ (L / 8 + 1) :=
      Bijou.bijou32.capacity_eq_pow (by omega)
    have hup : v < Bijou.bijou32.offset (L / 8 + 1 + 1) := by
      rw [Bijou.bijou32.offset_succ, hcap]
      omega
    have := Bijou.bijou32.tierOf_eq hv (by omega) hup
    omega

theorem leading_zeros_val {v : Std.U32} (hv : v.val ≠ 0) :
    (core.num.U32.leading_zeros v).val = 31 - Nat.log 2 v.val := by
  have hlog : Nat.log 2 v.val < 32 := by
    have := v.hBounds
    exact Nat.log_lt_of_lt_pow (by omega) (by simpa using this)
  simp only [core.num.U32.leading_zeros, BitVec.leadingZeros]
  split
  · rename_i h0
    exact absurd (by simp [UScalar.val, h0]) hv
  · have hl : Nat.log 2 v.bv.toNat < 32 := hlog
    have h32 : (2 : ℕ) ^ UScalarTy.U32.numBits = 4294967296 := rfl
    simp only [UScalar.val, BitVec.natCast_eq_ofNat, BitVec.toNat_ofNat, h32]
    generalize Nat.log 2 v.bv.toNat = L at *
    omega

theorem encoded_len_spec (v : Std.U32) :
    u32.encoded_len v ⦃ n => n.val = Bijou.bijou32.encodedLen v.val ⦄ := by
  unfold u32.encoded_len
  step*
  · have h := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h
    have hv : v.val < 252 := by scalar_tac
    simp [Bijou.Family.encodedLen, show Bijou.bijou32.threshold = 252 from rfl, hv]
  all_goals
    have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : 252 ≤ v.val := by scalar_tac
    have hlog : Nat.log 2 v.val < 32 :=
      Nat.log_lt_of_lt_pow (by omega) (by simpa using v.hBounds)
    have hlz : i1.val = 31 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
  · scalar_tac
  · scalar_tac
  · have hcand : candidate.val = Nat.log 2 v.val / 8 + 2 := by scalar_tac
    have hi5 : i5.val = Nat.log 2 v.val / 8 := by omega
    have hi6 : i6.val = Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this, hi5]
    have hlt : v.val < Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
    rw [tier_dispatch hv (by simpa using v.hBounds), ite_eq_left hlt]
    omega
  · have hcand : candidate.val = Nat.log 2 v.val / 8 + 2 := by scalar_tac
    have hi5 : i5.val = Nat.log 2 v.val / 8 := by omega
    have hi6 : i6.val = Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this, hi5]
    have hge : ¬ v.val < Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
    rw [tier_dispatch hv (by simpa using v.hBounds), ite_eq_right hge]
    omega


/-- The Rust's tier choice is the model's `tierOf`. -/
theorem tierOf_dispatch {v : ℕ} (hlo : 252 ≤ v) (hhi : v < 2 ^ 32) :
    Bijou.bijou32.tierOf v =
      if v < Bijou.bijou32.offset (Nat.log 2 v / 8 + 1) then Nat.log 2 v / 8
      else Nat.log 2 v / 8 + 1 := by
  have h := tier_dispatch hlo hhi
  have hnlt : ¬ v < Bijou.bijou32.threshold := by
    rw [show Bijou.bijou32.threshold = 252 from rfl]; omega
  simp only [Bijou.Family.encodedLen, hnlt, ite_false] at h
  split <;> split at h <;> omega

theorem tierOf_bounds {v : ℕ} (hlo : 252 ≤ v) (hhi : v < 2 ^ 32) :
    1 ≤ Bijou.bijou32.tierOf v ∧ Bijou.bijou32.tierOf v ≤ 4 := by
  have hv : v < 256 ^ Bijou.bijou32.tiers := by
    rw [show (256 : ℕ) ^ Bijou.bijou32.tiers = 2 ^ 32 by norm_num [Bijou.bijou32]]; exact hhi
  exact ⟨Bijou.bijou32.tierOf_pos (by rw [show Bijou.bijou32.threshold = 252 from rfl]; omega),
    Bijou.bijou32.tierOf_le v⟩

/-- The payload fits in `tierOf v` bytes. -/
theorem payload_lt {v : ℕ} (hlo : 252 ≤ v) (hhi : v < 2 ^ 32) :
    v - Bijou.bijou32.offset (Bijou.bijou32.tierOf v) < 256 ^ Bijou.bijou32.tierOf v := by
  have hv : v < 256 ^ Bijou.bijou32.tiers := by
    rw [show (256 : ℕ) ^ Bijou.bijou32.tiers = 2 ^ 32 by norm_num [Bijou.bijou32]]; exact hhi
  have h1 := Bijou.bijou32.lt_offset_tierOf_succ hv
  have h2 := Bijou.bijou32.offset_tierOf_le v
  have ht := (tierOf_bounds hlo hhi).1
  rw [Bijou.bijou32.offset_succ, Bijou.bijou32.capacity_eq_pow (by omega)] at h1
  omega

/-- The model's encoding of a multi-byte value. -/
theorem encode_multi {v : ℕ} (hlo : 252 ≤ v) :
    Bijou.bijou32.encode v = (252 + (Bijou.bijou32.tierOf v - 1)) ::
      Bijou.beBytes (Bijou.bijou32.tierOf v) (v - Bijou.bijou32.offset (Bijou.bijou32.tierOf v)) := by
  have hnlt : ¬ v < Bijou.bijou32.threshold := by
    rw [show Bijou.bijou32.threshold = 252 from rfl]; omega
  simp only [Bijou.Family.encode, hnlt, ite_false]
  rfl

@[step]
theorem OFFSETS_spec :
    u32.OFFSETS ⦃ a => ∀ i ≤ 4, (a.val[i]!).val = Bijou.bijou32.offset i ⦄ := by
  unfold u32.OFFSETS
  step*
  intro j hj
  interval_cases j <;> simp [*]

theorem encode_spec (v : Std.U32) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 5 ≤ Usize.max) :
    u32.encode v buf ⦃ out =>
      out.val.map (·.val) = buf.val.map (·.val) ++ Bijou.bijou32.encode v.val ⦄ := by
  unfold u32.encode
  step*
  · have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : v.val < 252 := by scalar_tac
    have hi1 : i1.val = v.val := by
      have h255 : (255#u32).bv.toNat = 2 ^ 8 - 1 := rfl
      have h := congrArg BitVec.toNat i1_post1
      rw [BitVec.toNat_and, h255, Nat.and_two_pow_sub_one_eq_mod] at h
      exact h.trans (by change v.val % 2 ^ 8 = v.val; omega)
    have hi2 : i2.val = v.val := by rw [i2_post]; scalar_tac
    rw [out_post, Bijou.bijou32.encode_lt_threshold
      (show v.val < Bijou.bijou32.threshold by rw [show Bijou.bijou32.threshold = 252 from rfl]; omega)]
    simp [hi2]
  all_goals
    have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : 252 ≤ v.val := by scalar_tac
    have hlz : i1.val = 31 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have hlog : Nat.log 2 v.val < 32 := Nat.log_lt_of_lt_pow (by omega) (by simpa using v.hBounds)
  · scalar_tac
  · scalar_tac
  · have hnt : u32.NUM_TIERS.val = 4 := by unfold u32.NUM_TIERS; simp
    have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : 252 ≤ v.val := by scalar_tac
    have hhi : v.val < 2 ^ 32 := by simpa using v.hBounds
    have hlog : Nat.log 2 v.val < 32 := Nat.log_lt_of_lt_pow (by omega) hhi
    have hlz : i1.val = 31 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have htier : tier.val = Nat.log 2 v.val / 8 + 1 := by scalar_tac
    have hi6 : i6.val = Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this]
      congr 1; omega
    obtain ⟨ht1, ht8⟩ := tierOf_bounds hv hhi
    split
    · rename_i hlt
      have hlt' : v.val < Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : i5.val = Bijou.bijou32.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_left hlt']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou32.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou32.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou32.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (4 - Bijou.bijou32.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (4 - Bijou.bijou32.tierOf v.val)) =
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) < 2 ^ 32 := by
            calc _ < 256 ^ Bijou.bijou32.tierOf v.val * 256 ^ (4 - Bijou.bijou32.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 32 := by rw [← pow_add, show Bijou.bijou32.tierOf v.val +
                  (4 - Bijou.bijou32.tierOf v.val) = 4 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U32.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 252 := by
          rw [i7_post]; unfold u32.TAG_THRESHOLD; simp
        have htag : tag.val = 251 + Bijou.bijou32.tierOf v.val := by
          scalar_tac
        have hlen : i19.val = buf.val.length + Bijou.bijou32.tierOf v.val + 1 := by
          rw [i19_post, i18_post, ht]; simp
        have hpb : pb.val.length = 4 := by simp
        rw [encode_multi hv, out_post, buf1_post, s_post]
        simp only [Array.val_to_slice, Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post,
          four_elems pb.val hpb, hlen, take_assemble (fun x => x.val) buf.val tag pb.val,
          htag, show 251 + Bijou.bijou32.tierOf v.val = 252 + (Bijou.bijou32.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl
    · rename_i hge
      have hge' : ¬ v.val < Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : tier.val = Bijou.bijou32.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_right hge']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou32.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou32.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou32.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (4 - Bijou.bijou32.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (4 - Bijou.bijou32.tierOf v.val)) =
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) < 2 ^ 32 := by
            calc _ < 256 ^ Bijou.bijou32.tierOf v.val * 256 ^ (4 - Bijou.bijou32.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 32 := by rw [← pow_add, show Bijou.bijou32.tierOf v.val +
                  (4 - Bijou.bijou32.tierOf v.val) = 4 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U32.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 252 := by
          rw [i7_post]; unfold u32.TAG_THRESHOLD; simp
        have htag : tag.val = 251 + Bijou.bijou32.tierOf v.val := by
          scalar_tac
        have hlen : i19.val = buf.val.length + Bijou.bijou32.tierOf v.val + 1 := by
          rw [i19_post, i18_post, ht]; simp
        have hpb : pb.val.length = 4 := by simp
        rw [encode_multi hv, out_post, buf1_post, s_post]
        simp only [Array.val_to_slice, Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post,
          four_elems pb.val hpb, hlen, take_assemble (fun x => x.val) buf.val tag pb.val,
          htag, show 251 + Bijou.bijou32.tierOf v.val = 252 + (Bijou.bijou32.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl

/-! ## `encoded_bytes`: the same bytes, without allocating -/

/-- `as_slice` returns the first `len` bytes of `buf`. -/
@[step]
theorem as_slice_spec (e : u32.EncodedU32) (h : e.len.val ≤ 5) :
    u32.EncodedU32.as_slice e ⦃ s => s.val = e.buf.val.take e.len.val ⦄ := by
  unfold u32.EncodedU32.as_slice
  step*

-- Same proof shape as `encode_spec` plus the `len` field: a bit over the
-- default heartbeat budget.
set_option maxHeartbeats 400000 in
/-- `encoded_bytes v` holds the model's `encode v` in its first `len` bytes. -/
theorem encoded_bytes_spec (v : Std.U32) :
    u32.encoded_bytes v ⦃ e => e.len.val = Bijou.bijou32.encodedLen v.val ∧
      (e.buf.val.take e.len.val).map (·.val) = Bijou.bijou32.encode v.val ⦄ := by
  unfold u32.encoded_bytes
  step*
  · have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : v.val < 252 := by scalar_tac
    have hi1 : i1.val = v.val := by
      have h255 : (255#u32).bv.toNat = 2 ^ 8 - 1 := rfl
      have h := congrArg BitVec.toNat i1_post1
      rw [BitVec.toNat_and, h255, Nat.and_two_pow_sub_one_eq_mod] at h
      exact h.trans (by change v.val % 2 ^ 8 = v.val; omega)
    have hi2 : i2.val = v.val := by rw [i2_post]; scalar_tac
    have hlt : v.val < Bijou.bijou32.threshold := by
      rw [show Bijou.bijou32.threshold = 252 from rfl]; omega
    rw [Bijou.bijou32.encode_lt_threshold hlt]
    simp [Bijou.Family.encodedLen, hlt, hi2, Array.make]
  all_goals
    have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : 252 ≤ v.val := by scalar_tac
    have hlz : i1.val = 31 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have hlog : Nat.log 2 v.val < 32 := Nat.log_lt_of_lt_pow (by omega) (by simpa using v.hBounds)
  · scalar_tac
  · scalar_tac
  · have hnt : u32.NUM_TIERS.val = 4 := by unfold u32.NUM_TIERS; simp
    have h0 := a_post 0 (by omega)
    have hi : i.val = 252 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou32.threshold = 252 from rfl] using h0
    have hv : 252 ≤ v.val := by scalar_tac
    have hhi : v.val < 2 ^ 32 := by simpa using v.hBounds
    have hlog : Nat.log 2 v.val < 32 := Nat.log_lt_of_lt_pow (by omega) hhi
    have hlz : i1.val = 31 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have htier : tier.val = Nat.log 2 v.val / 8 + 1 := by scalar_tac
    have hi6 : i6.val = Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this]
      congr 1; omega
    obtain ⟨ht1, ht8⟩ := tierOf_bounds hv hhi
    split
    · rename_i hlt
      have hlt' : v.val < Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : i5.val = Bijou.bijou32.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_left hlt']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou32.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou32.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou32.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (4 - Bijou.bijou32.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (4 - Bijou.bijou32.tierOf v.val)) =
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) < 2 ^ 32 := by
            calc _ < 256 ^ Bijou.bijou32.tierOf v.val * 256 ^ (4 - Bijou.bijou32.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 32 := by rw [← pow_add, show Bijou.bijou32.tierOf v.val +
                  (4 - Bijou.bijou32.tierOf v.val) = 4 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U32.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 252 := by
          rw [i7_post]; unfold u32.TAG_THRESHOLD; simp
        have htag : tag.val = 251 + Bijou.bijou32.tierOf v.val := by
          scalar_tac
        have hlen : i19.val = Bijou.bijou32.tierOf v.val + 1 := by
          have hi22 : i18.val = Bijou.bijou32.tierOf v.val + 1 := by rw [i18_post, ht]
          rw [i19_post]
          scalar_tac
        have hpb : pb.val.length = 4 := by simp
        have hnlt : ¬ v.val < Bijou.bijou32.threshold := by
          rw [show Bijou.bijou32.threshold = 252 from rfl]; omega
        refine ⟨by simp [hlen, Bijou.Family.encodedLen, hnlt], ?_⟩
        rw [encode_multi hv]
        simp only [Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post,
          four_elems pb.val hpb, hlen, List.take_succ_cons, List.map_cons, htag,
          show 251 + Bijou.bijou32.tierOf v.val = 252 + (Bijou.bijou32.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl
    · rename_i hge
      have hge' : ¬ v.val < Bijou.bijou32.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : tier.val = Bijou.bijou32.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_right hge']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou32.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou32.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou32.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (4 - Bijou.bijou32.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (4 - Bijou.bijou32.tierOf v.val)) =
              256 ^ (4 - Bijou.bijou32.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou32.offset (Bijou.bijou32.tierOf v.val)) *
              256 ^ (4 - Bijou.bijou32.tierOf v.val) < 2 ^ 32 := by
            calc _ < 256 ^ Bijou.bijou32.tierOf v.val * 256 ^ (4 - Bijou.bijou32.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 32 := by rw [← pow_add, show Bijou.bijou32.tierOf v.val +
                  (4 - Bijou.bijou32.tierOf v.val) = 4 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U32.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 252 := by
          rw [i7_post]; unfold u32.TAG_THRESHOLD; simp
        have htag : tag.val = 251 + Bijou.bijou32.tierOf v.val := by
          scalar_tac
        have hlen : i19.val = Bijou.bijou32.tierOf v.val + 1 := by
          have hi22 : i18.val = Bijou.bijou32.tierOf v.val + 1 := by rw [i18_post, ht]
          rw [i19_post]
          scalar_tac
        have hpb : pb.val.length = 4 := by simp
        have hnlt : ¬ v.val < Bijou.bijou32.threshold := by
          rw [show Bijou.bijou32.threshold = 252 from rfl]; omega
        refine ⟨by simp [hlen, Bijou.Family.encodedLen, hnlt], ?_⟩
        rw [encode_multi hv]
        simp only [Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post,
          four_elems pb.val hpb, hlen, List.take_succ_cons, List.map_cons, htag,
          show 251 + Bijou.bijou32.tierOf v.val = 252 + (Bijou.bijou32.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl


/-- What callers see: `encoded_bytes(v).as_slice()` is exactly the model's
encoding of `v`, the same bytes `encode` appends. -/
theorem encoded_bytes_as_slice (v : Std.U32) :
    u32.encoded_bytes v ⦃ e => u32.EncodedU32.as_slice e ⦃ s =>
      s.val.map (·.val) = Bijou.bijou32.encode v.val ⦄ ⦄ := by
  apply WP.spec_mono (encoded_bytes_spec v)
  rintro e ⟨hl, hb⟩
  have hle : e.len.val ≤ 5 := by
    rw [hl]; exact Bijou.bijou32.encodedLen_le v.val
  apply WP.spec_mono (as_slice_spec e hle)
  intro s hs
  rw [hs, hb]

/-- The Rust decode result, read as the model's. -/
def toModel : core.result.Result (Std.U32 × Std.Usize) u32.DecodeError →
    Except Bijou.DecodeError (ℕ × ℕ)
  | .Ok (v, n) => .ok (v.val, n.val)
  | .Err .BufferTooShort => .error .bufferTooShort
  | .Err .Overflow => .error .overflow

theorem model_decode_multi (tag : ℕ) (rest : List ℕ) (h1 : 252 ≤ tag) :
    Bijou.bijou32.decode (tag :: rest) =
      if tag - 251 ≤ rest.length then
        if Bijou.bijou32.offset (tag - 251) + Bijou.fromBe (rest.take (tag - 251)) < 2 ^ 32 then
          .ok (Bijou.bijou32.offset (tag - 251) + Bijou.fromBe (rest.take (tag - 251)), tag - 250)
        else .error .overflow
      else .error .bufferTooShort := by
  have hthr : Bijou.bijou32.threshold = 252 := rfl
  have hpow : (256 : ℕ) ^ Bijou.bijou32.tiers = 2 ^ 32 := by norm_num [Bijou.bijou32]
  simp only [Bijou.Family.decode, hthr, show ¬ tag < 252 by omega, ite_false, hpow,
    show tag - 252 + 1 = tag - 251 by omega, show tag - 252 + 2 = tag - 250 by omega]

-- Generated by scripts/gen_widths.py from its `decode` template, for every
-- width (one arm per tier). Edit the template, not this proof.
theorem decode_spec (buf : Slice Std.U8) :
    u32.decode buf ⦃ r => toModel r = Bijou.bijou32.decode (buf.val.map (·.val)) ⦄ := by
  unfold u32.decode
  step*
  · rename_i h
    rw [h] at o_post
    simp only at o_post
    simp [o_post, toModel, Bijou.Family.decode]
  · rename_i h
    rw [h] at o_post
    cases p with
    | mk tag rest =>
    simp only at o_post
    change (if tag < u32.TAG_THRESHOLD then _ else _) ⦃ r =>
      toModel r = Bijou.bijou32.decode (buf.val.map (·.val)) ⦄
    split
    · step*
      have hlt' : tag.val < 252 := by
        have : u32.TAG_THRESHOLD.val = 252 := by unfold u32.TAG_THRESHOLD; rfl
        scalar_tac
      simp [o_post, toModel, Bijou.Family.decode, show Bijou.bijou32.threshold = 252 from rfl,
        hlt', i_post]
    · rename_i hge
      split
      · step*
        · rename_i hn
          rw [hn] at o1_post
          simp only at o1_post
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide)]
          have htv : (↑(252#8#uscalar : Std.U8) : ℕ) = 252 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(252#8#uscalar : Std.U8) : ℕ) = 252 := rfl
          have hoff : offset.val = Bijou.bijou32.offset 1 := by
            have := a2_post 1 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 1).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 1 := by have := a.property; simp_all
                interval_cases idx
                simp [a1_post]
            have hz := fromBe_zeros 3 [a1.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U32.max = 2 ^ 32 - 1 := by rw [U32.max_eq]; norm_num
          first
          | rw [‹o2 = none›] at o2_post
            simp only at o2_post
            rw [ite_eq_right (by omega)]
            rfl
          | rw [‹o2 = some value›] at o2_post
            obtain ⟨hle, hval, _⟩ := o2_post
            rw [ite_eq_left (by omega)]
            simp [toModel, hval]
      · step*
        · rename_i hn
          rw [hn] at o1_post
          simp only at o1_post
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide)]
          have htv : (↑(253#8#uscalar : Std.U8) : ℕ) = 253 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(253#8#uscalar : Std.U8) : ℕ) = 253 := rfl
          have hoff : offset.val = Bijou.bijou32.offset 2 := by
            have := a2_post 2 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 2).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 2 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post]
            have hz := fromBe_zeros 2 [a1.val, b.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U32.max = 2 ^ 32 - 1 := by rw [U32.max_eq]; norm_num
          first
          | rw [‹o2 = none›] at o2_post
            simp only at o2_post
            rw [ite_eq_right (by omega)]
            rfl
          | rw [‹o2 = some value›] at o2_post
            obtain ⟨hle, hval, _⟩ := o2_post
            rw [ite_eq_left (by omega)]
            simp [toModel, hval]
      · step*
        · rename_i hn
          rw [hn] at o1_post
          simp only at o1_post
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide)]
          have htv : (↑(254#8#uscalar : Std.U8) : ℕ) = 254 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(254#8#uscalar : Std.U8) : ℕ) = 254 := rfl
          have hoff : offset.val = Bijou.bijou32.offset 3 := by
            have := a2_post 3 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 3).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 3 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post]
            have hz := fromBe_zeros 1 [a1.val, b.val, c.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U32.max = 2 ^ 32 - 1 := by rw [U32.max_eq]; norm_num
          first
          | rw [‹o2 = none›] at o2_post
            simp only at o2_post
            rw [ite_eq_right (by omega)]
            rfl
          | rw [‹o2 = some value›] at o2_post
            obtain ⟨hle, hval, _⟩ := o2_post
            rw [ite_eq_left (by omega)]
            simp [toModel, hval]
      · rename_i x252 x253 x254
        have htt : u32.TAG_THRESHOLD.val = 252 := by unfold u32.TAG_THRESHOLD; rfl
        have hn252 : tag.val ≠ 252 := fun hv =>
          x252 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn253 : tag.val ≠ 253 := fun hv =>
          x253 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn254 : tag.val ≠ 254 := fun hv =>
          x254 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hge' : 252 ≤ tag.val := by scalar_tac
        have hlt' : tag.val < 256 := by have := tag.hBounds; simpa using this
        have htv : tag.val = 255 := by omega
        step*
        · rename_i hn
          rw [hn] at o1_post
          simp only at o1_post
          rw [o_post, List.map_cons, model_decode_multi _ _ (by omega)]
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have hoff : offset.val = Bijou.bijou32.offset 4 := by
            have := a2_post 4 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 4).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 4 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post]
            rw [← ha, hlist]
            simp [Array.make, Array.from_val, Function.comp]
          rw [o_post, List.map_cons, model_decode_multi _ _ (by omega), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U32.max = 2 ^ 32 - 1 := by rw [U32.max_eq]; norm_num
          first
          | rw [‹o2 = none›] at o2_post
            simp only at o2_post
            rw [ite_eq_right (by omega)]
            rfl
          | rw [‹o2 = some value›] at o2_post
            obtain ⟨hle, hval, _⟩ := o2_post
            rw [ite_eq_left (by omega)]
            simp [toModel, hval]

end BijouAeneas.U32
