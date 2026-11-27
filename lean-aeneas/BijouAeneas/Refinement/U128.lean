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
# `bijoux::u128` refines the bijou128 SPEC model

Each theorem states that a function translated from the real Rust
(`BijouAeneas.Generated`) never panics and returns exactly what the
corresponding `Bijou.Family` definition prescribes for `Bijou.bijou128`.

The proofs are `U64.lean`'s, with the width constants and Aeneas's
variable names substituted (`scripts/gen_widths.py`). Change `U64.lean`
first, then regenerate this file rather than editing it by hand.

- `tier_offset_spec` — the `tier_offset` loop computes the model's tier
  offsets; its saturating arithmetic never saturates for tiers `≤ 16`.
- `BOUNDS_spec` — so the `BOUNDS` table holds `offset 1 .. offset 16`.
- `tier_dispatch` — the `leading_zeros` tier dispatch (`(bw - 1) / 8 + 2`,
  corrected by one `BOUNDS` comparison) always lands on the model's tier.
- `encoded_len_spec` — `encoded_len v = encodedLen v` for every `u128`.
- `encode_spec` — `encode v buf` appends exactly the model's `encode v`.
- `encoded_bytes_as_slice` — `encoded_bytes(v).as_slice()` is exactly the
  model's `encode v` (via `encoded_bytes_spec` and `as_slice_spec`).
- `decode_spec` — on *every* input, `decode` returns exactly the model's
  `decode` (same value and length, or the same error) and never panics.
-/

open Aeneas Aeneas.Std Result
open BijouRust

namespace BijouAeneas.U128

theorem sat_mul_val (x y : Std.U128) (h : x.val * y.val ≤ U128.max) :
    core.num.U128.saturating_mul x y ⦃ z => z.val = x.val * y.val ⦄ := by
  have hW : (2 : ℕ) ^ UScalarTy.U128.numBits = 340282366920938463463374607431768211456 := rfl
  rw [U128.max_eq] at h
  simp only [core.num.U128.saturating_mul, WP.spec_ok, UScalar.max_UScalarTy_U128_eq, U128.max_eq,
    UScalar.val, BitVec.toNat_ofNat, hW] at h ⊢
  omega

theorem sat_add_val (x y : Std.U128) (h : x.val + y.val ≤ U128.max) :
    (core.num.U128.saturating_add x y).val = x.val + y.val := by
  have hW : (2 : ℕ) ^ UScalarTy.U128.numBits = 340282366920938463463374607431768211456 := rfl
  rw [U128.max_eq] at h
  simp only [core.num.U128.saturating_add, UScalar.saturating_add,
    UScalar.max_UScalarTy_U128_eq, U128.max_eq,
    UScalar.val, BitVec.toNat_ofNat, hW] at h ⊢
  omega

theorem tier_offset_loop_spec (n : Std.Usize) (hn : n.val ≤ 16)
    (result power : Std.U128) (i : Std.Usize)
    (hi : 2 ≤ i.val) (hin : i.val ≤ n.val + 1)
    (hr : result.val = Bijou.bijou128.offset (i.val - 1))
    (hp : power.val = 256 ^ (i.val - 2)) :
    u128.tier_offset_loop n result power i ⦃ r => r.val = Bijou.bijou128.offset n.val ⦄ := by
  unfold u128.tier_offset_loop
  apply loop.spec_decr_nat (fun (x : Std.U128 × Std.U128 × Std.Usize) => n.val + 1 - x.2.2.val)
    (fun x => 2 ≤ x.2.2.val ∧ x.2.2.val ≤ n.val + 1 ∧
      x.1.val = Bijou.bijou128.offset (x.2.2.val - 1) ∧ x.2.1.val = 256 ^ (x.2.2.val - 2))
  · rintro ⟨r, p, j⟩ ⟨hj2, hjn, hr, hp⟩
    simp only [u128.tier_offset_loop.body]
    split
    · rename_i hle
      simp only at hj2 hjn hr hp
      have hjn' : j.val ≤ n.val := by scalar_tac
      have hp256 : p.val * 256 = 256 ^ (j.val - 1) := by
        rw [hp, ← pow_succ]
        congr 1
        omega
      have hplt : 256 ^ (j.val - 1) ≤ U128.max := by
        rw [U128.max_eq]
        calc 256 ^ (j.val - 1) ≤ 256 ^ 15 := Nat.pow_le_pow_right (by norm_num) (by omega)
          _ ≤ 340282366920938463463374607431768211455 := by norm_num
      have hoff : Bijou.bijou128.offset (j.val - 1) + 256 ^ (j.val - 1) =
          Bijou.bijou128.offset j.val := by
        have h := Bijou.bijou128.offset_succ (j.val - 1)
        rw [Bijou.bijou128.capacity_eq_pow (by omega), show j.val - 1 + 1 = j.val by omega] at h
        omega
      have hofflt : Bijou.bijou128.offset j.val ≤ U128.max := by
        have h := Bijou.bijou128.offset_lt_pow (t := j.val) (by omega)
        have h8 : (256 : ℕ) ^ j.val ≤ 256 ^ 16 := Nat.pow_le_pow_right (by norm_num) (by omega)
        rw [U128.max_eq]
        have : (256 : ℕ) ^ 16 = 340282366920938463463374607431768211456 := by norm_num
        omega
      step with sat_mul_val p 256#u128 (by simp; omega)
      simp only [lift]
      step*
      have hsum : r.val + power1.val ≤ U128.max := by
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
    (UScalar.cast UScalarTy.U128 u128.TAG_THRESHOLD).val = 240 := by
  unfold u128.TAG_THRESHOLD
  simp

@[step]
theorem tier_offset_spec (n : Std.Usize) (h : n.val ≤ 16) :
    u128.tier_offset n ⦃ r => r.val = Bijou.bijou128.offset n.val ⦄ := by
  unfold u128.tier_offset
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
      step with tier_offset_loop_spec n h _ 1#u128 2#usize (by simp) (by simp; omega)
        (by rw [tag_threshold_cast]; rfl) (by simp)
      assumption

@[step]
theorem BOUNDS_spec :
    u128.BOUNDS ⦃ a => ∀ i < 16, (a.val[i]!).val = Bijou.bijou128.offset (i + 1) ⦄ := by
  unfold u128.BOUNDS
  step*
  intro j hj
  interval_cases j <;> simp [*]

/-- The Rust tier dispatch, as a statement about the model: with
`L = ⌊log₂ v⌋` (so the byte length is `L / 8 + 1`), the encoded length is
that byte length if `v` is below the next tier's offset, else one more. -/
theorem tier_dispatch {v : ℕ} (hlo : 240 ≤ v) (hhi : v < 2 ^ 128) :
    Bijou.bijou128.encodedLen v =
      if v < Bijou.bijou128.offset (Nat.log 2 v / 8 + 1) then Nat.log 2 v / 8 + 1
      else Nat.log 2 v / 8 + 2 := by
  have hthr : Bijou.bijou128.threshold = 240 := rfl
  have htiers : (256 : ℕ) ^ Bijou.bijou128.tiers = 2 ^ 128 := by norm_num [Bijou.bijou128]
  have hnlt : ¬ v < Bijou.bijou128.threshold := by omega
  simp only [Bijou.Family.encodedLen, hnlt, ite_false]
  set L := Nat.log 2 v with hL
  have h1 : 2 ^ L ≤ v := Nat.pow_log_le_self 2 (by omega)
  have h2 : v < 2 ^ (L + 1) := Nat.lt_pow_succ_log_self (by norm_num) v
  have hv : v < 256 ^ Bijou.bijou128.tiers := by rw [htiers]; exact hhi
  split
  · rename_i hlt
    have hlow : Bijou.bijou128.offset (L / 8) ≤ v := by
      rcases Nat.eq_zero_or_pos (L / 8) with h0 | hpos
      · rw [h0]; simp
      · have hoff := Bijou.bijou128.offset_lt_pow hpos
        have hpow : 256 ^ (L / 8) ≤ 2 ^ L := by
          rw [show (256 : ℕ) = 2 ^ 8 by norm_num, ← pow_mul]
          exact Nat.pow_le_pow_right (by norm_num) (by omega)
        omega
    have := Bijou.bijou128.tierOf_eq hv hlow hlt
    omega
  · rename_i hge
    have hpow : 2 ^ (L + 1) ≤ 256 ^ (L / 8 + 1) := by
      rw [show (256 : ℕ) = 2 ^ 8 by norm_num, ← pow_mul]
      exact Nat.pow_le_pow_right (by norm_num) (by omega)
    have hcap : Bijou.bijou128.capacity (L / 8 + 1) = 256 ^ (L / 8 + 1) :=
      Bijou.bijou128.capacity_eq_pow (by omega)
    have hup : v < Bijou.bijou128.offset (L / 8 + 1 + 1) := by
      rw [Bijou.bijou128.offset_succ, hcap]
      omega
    have := Bijou.bijou128.tierOf_eq hv (by omega) hup
    omega

theorem leading_zeros_val {v : Std.U128} (hv : v.val ≠ 0) :
    (core.num.U128.leading_zeros v).val = 127 - Nat.log 2 v.val := by
  have hlog : Nat.log 2 v.val < 128 := by
    have := v.hBounds
    exact Nat.log_lt_of_lt_pow (by omega) (by simpa using this)
  simp only [core.num.U128.leading_zeros, BitVec.leadingZeros]
  split
  · rename_i h0
    exact absurd (by simp [UScalar.val, h0]) hv
  · have hl : Nat.log 2 v.bv.toNat < 128 := hlog
    have h32 : (2 : ℕ) ^ UScalarTy.U32.numBits = 4294967296 := rfl
    simp only [UScalar.val, BitVec.natCast_eq_ofNat, BitVec.toNat_ofNat, h32]
    generalize Nat.log 2 v.bv.toNat = L at *
    omega

theorem encoded_len_spec (v : Std.U128) :
    u128.encoded_len v ⦃ n => n.val = Bijou.bijou128.encodedLen v.val ⦄ := by
  unfold u128.encoded_len
  step*
  · have h := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h
    have hv : v.val < 240 := by scalar_tac
    simp [Bijou.Family.encodedLen, show Bijou.bijou128.threshold = 240 from rfl, hv]
  all_goals
    have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : 240 ≤ v.val := by scalar_tac
    have hlog : Nat.log 2 v.val < 128 :=
      Nat.log_lt_of_lt_pow (by omega) (by simpa using v.hBounds)
    have hlz : i1.val = 127 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
  · scalar_tac
  · scalar_tac
  · have hcand : candidate.val = Nat.log 2 v.val / 8 + 2 := by scalar_tac
    have hi5 : i5.val = Nat.log 2 v.val / 8 := by omega
    have hi6 : i6.val = Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this, hi5]
    have hlt : v.val < Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
    rw [tier_dispatch hv (by simpa using v.hBounds), ite_eq_left hlt]
    omega
  · have hcand : candidate.val = Nat.log 2 v.val / 8 + 2 := by scalar_tac
    have hi5 : i5.val = Nat.log 2 v.val / 8 := by omega
    have hi6 : i6.val = Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this, hi5]
    have hge : ¬ v.val < Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
    rw [tier_dispatch hv (by simpa using v.hBounds), ite_eq_right hge]
    omega


/-- The Rust's tier choice is the model's `tierOf`. -/
theorem tierOf_dispatch {v : ℕ} (hlo : 240 ≤ v) (hhi : v < 2 ^ 128) :
    Bijou.bijou128.tierOf v =
      if v < Bijou.bijou128.offset (Nat.log 2 v / 8 + 1) then Nat.log 2 v / 8
      else Nat.log 2 v / 8 + 1 := by
  have h := tier_dispatch hlo hhi
  have hnlt : ¬ v < Bijou.bijou128.threshold := by
    rw [show Bijou.bijou128.threshold = 240 from rfl]; omega
  simp only [Bijou.Family.encodedLen, hnlt, ite_false] at h
  split <;> split at h <;> omega

theorem tierOf_bounds {v : ℕ} (hlo : 240 ≤ v) (hhi : v < 2 ^ 128) :
    1 ≤ Bijou.bijou128.tierOf v ∧ Bijou.bijou128.tierOf v ≤ 16 := by
  have hv : v < 256 ^ Bijou.bijou128.tiers := by
    rw [show (256 : ℕ) ^ Bijou.bijou128.tiers = 2 ^ 128 by norm_num [Bijou.bijou128]]; exact hhi
  exact ⟨Bijou.bijou128.tierOf_pos (by rw [show Bijou.bijou128.threshold = 240 from rfl]; omega),
    Bijou.bijou128.tierOf_le v⟩

/-- The payload fits in `tierOf v` bytes. -/
theorem payload_lt {v : ℕ} (hlo : 240 ≤ v) (hhi : v < 2 ^ 128) :
    v - Bijou.bijou128.offset (Bijou.bijou128.tierOf v) < 256 ^ Bijou.bijou128.tierOf v := by
  have hv : v < 256 ^ Bijou.bijou128.tiers := by
    rw [show (256 : ℕ) ^ Bijou.bijou128.tiers = 2 ^ 128 by norm_num [Bijou.bijou128]]; exact hhi
  have h1 := Bijou.bijou128.lt_offset_tierOf_succ hv
  have h2 := Bijou.bijou128.offset_tierOf_le v
  have ht := (tierOf_bounds hlo hhi).1
  rw [Bijou.bijou128.offset_succ, Bijou.bijou128.capacity_eq_pow (by omega)] at h1
  omega

/-- The model's encoding of a multi-byte value. -/
theorem encode_multi {v : ℕ} (hlo : 240 ≤ v) :
    Bijou.bijou128.encode v = (240 + (Bijou.bijou128.tierOf v - 1)) ::
      Bijou.beBytes (Bijou.bijou128.tierOf v) (v - Bijou.bijou128.offset (Bijou.bijou128.tierOf v)) := by
  have hnlt : ¬ v < Bijou.bijou128.threshold := by
    rw [show Bijou.bijou128.threshold = 240 from rfl]; omega
  simp only [Bijou.Family.encode, hnlt, ite_false]
  rfl

@[step]
theorem OFFSETS_spec :
    u128.OFFSETS ⦃ a => ∀ i ≤ 16, (a.val[i]!).val = Bijou.bijou128.offset i ⦄ := by
  unfold u128.OFFSETS
  step*
  intro j hj
  interval_cases j <;> simp [*]

-- 16 tiers: twice the bytes (and `decode` arms) of `u64`, so these two
-- whole-function proofs need a larger heartbeat budget than the default.
set_option maxHeartbeats 1000000 in
theorem encode_spec (v : Std.U128) (buf : alloc.vec.Vec Std.U8)
    (hbuf : buf.length + 17 ≤ Usize.max) :
    u128.encode v buf ⦃ out =>
      out.val.map (·.val) = buf.val.map (·.val) ++ Bijou.bijou128.encode v.val ⦄ := by
  unfold u128.encode
  step*
  · have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : v.val < 240 := by scalar_tac
    have hi1 : i1.val = v.val := by
      have h255 : (255#u128).bv.toNat = 2 ^ 8 - 1 := rfl
      have h := congrArg BitVec.toNat i1_post1
      rw [BitVec.toNat_and, h255, Nat.and_two_pow_sub_one_eq_mod] at h
      exact h.trans (by change v.val % 2 ^ 8 = v.val; omega)
    have hi2 : i2.val = v.val := by rw [i2_post]; scalar_tac
    rw [out_post, Bijou.bijou128.encode_lt_threshold
      (show v.val < Bijou.bijou128.threshold by rw [show Bijou.bijou128.threshold = 240 from rfl]; omega)]
    simp [hi2]
  all_goals
    have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : 240 ≤ v.val := by scalar_tac
    have hlz : i1.val = 127 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have hlog : Nat.log 2 v.val < 128 := Nat.log_lt_of_lt_pow (by omega) (by simpa using v.hBounds)
  · scalar_tac
  · scalar_tac
  · have hnt : u128.NUM_TIERS.val = 16 := by unfold u128.NUM_TIERS; simp
    have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : 240 ≤ v.val := by scalar_tac
    have hhi : v.val < 2 ^ 128 := by simpa using v.hBounds
    have hlog : Nat.log 2 v.val < 128 := Nat.log_lt_of_lt_pow (by omega) hhi
    have hlz : i1.val = 127 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have htier : tier.val = Nat.log 2 v.val / 8 + 1 := by scalar_tac
    have hi6 : i6.val = Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this]
      congr 1; omega
    obtain ⟨ht1, ht8⟩ := tierOf_bounds hv hhi
    split
    · rename_i hlt
      have hlt' : v.val < Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : i5.val = Bijou.bijou128.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_left hlt']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou128.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou128.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou128.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (16 - Bijou.bijou128.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (16 - Bijou.bijou128.tierOf v.val)) =
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) < 2 ^ 128 := by
            calc _ < 256 ^ Bijou.bijou128.tierOf v.val * 256 ^ (16 - Bijou.bijou128.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 128 := by rw [← pow_add, show Bijou.bijou128.tierOf v.val +
                  (16 - Bijou.bijou128.tierOf v.val) = 16 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U128.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 240 := by
          rw [i7_post]; unfold u128.TAG_THRESHOLD; simp
        have htag : tag.val = 239 + Bijou.bijou128.tierOf v.val := by
          scalar_tac
        have hlen : i31.val = buf.val.length + Bijou.bijou128.tierOf v.val + 1 := by
          rw [i31_post, i30_post, ht]; simp
        have hpb : pb.val.length = 16 := by simp
        rw [encode_multi hv, out_post, buf1_post, s_post]
        simp only [Array.val_to_slice, Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post, i18_post, i19_post, i20_post, i21_post, i22_post, i23_post, i24_post, i25_post, i26_post, i27_post, i28_post, i29_post,
          sixteen_elems pb.val hpb, hlen, take_assemble (fun x => x.val) buf.val tag pb.val,
          htag, show 239 + Bijou.bijou128.tierOf v.val = 240 + (Bijou.bijou128.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl
    · rename_i hge
      have hge' : ¬ v.val < Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : tier.val = Bijou.bijou128.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_right hge']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou128.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou128.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou128.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (16 - Bijou.bijou128.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (16 - Bijou.bijou128.tierOf v.val)) =
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) < 2 ^ 128 := by
            calc _ < 256 ^ Bijou.bijou128.tierOf v.val * 256 ^ (16 - Bijou.bijou128.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 128 := by rw [← pow_add, show Bijou.bijou128.tierOf v.val +
                  (16 - Bijou.bijou128.tierOf v.val) = 16 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U128.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 240 := by
          rw [i7_post]; unfold u128.TAG_THRESHOLD; simp
        have htag : tag.val = 239 + Bijou.bijou128.tierOf v.val := by
          scalar_tac
        have hlen : i31.val = buf.val.length + Bijou.bijou128.tierOf v.val + 1 := by
          rw [i31_post, i30_post, ht]; simp
        have hpb : pb.val.length = 16 := by simp
        rw [encode_multi hv, out_post, buf1_post, s_post]
        simp only [Array.val_to_slice, Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post, i18_post, i19_post, i20_post, i21_post, i22_post, i23_post, i24_post, i25_post, i26_post, i27_post, i28_post, i29_post,
          sixteen_elems pb.val hpb, hlen, take_assemble (fun x => x.val) buf.val tag pb.val,
          htag, show 239 + Bijou.bijou128.tierOf v.val = 240 + (Bijou.bijou128.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl

/-! ## `encoded_bytes`: the same bytes, without allocating -/

/-- `as_slice` returns the first `len` bytes of `buf`. -/
@[step]
theorem as_slice_spec (e : u128.EncodedU128) (h : e.len.val ≤ 17) :
    u128.EncodedU128.as_slice e ⦃ s => s.val = e.buf.val.take e.len.val ⦄ := by
  unfold u128.EncodedU128.as_slice
  step*

-- Same proof shape as `encode_spec` plus the `len` field: a bit over the
-- default heartbeat budget.
set_option maxHeartbeats 2000000 in
/-- `encoded_bytes v` holds the model's `encode v` in its first `len` bytes. -/
theorem encoded_bytes_spec (v : Std.U128) :
    u128.encoded_bytes v ⦃ e => e.len.val = Bijou.bijou128.encodedLen v.val ∧
      (e.buf.val.take e.len.val).map (·.val) = Bijou.bijou128.encode v.val ⦄ := by
  unfold u128.encoded_bytes
  step*
  · have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : v.val < 240 := by scalar_tac
    have hi1 : i1.val = v.val := by
      have h255 : (255#u128).bv.toNat = 2 ^ 8 - 1 := rfl
      have h := congrArg BitVec.toNat i1_post1
      rw [BitVec.toNat_and, h255, Nat.and_two_pow_sub_one_eq_mod] at h
      exact h.trans (by change v.val % 2 ^ 8 = v.val; omega)
    have hi2 : i2.val = v.val := by rw [i2_post]; scalar_tac
    have hlt : v.val < Bijou.bijou128.threshold := by
      rw [show Bijou.bijou128.threshold = 240 from rfl]; omega
    rw [Bijou.bijou128.encode_lt_threshold hlt]
    simp [Bijou.Family.encodedLen, hlt, hi2, Array.make]
  all_goals
    have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : 240 ≤ v.val := by scalar_tac
    have hlz : i1.val = 127 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have hlog : Nat.log 2 v.val < 128 := Nat.log_lt_of_lt_pow (by omega) (by simpa using v.hBounds)
  · scalar_tac
  · scalar_tac
  · have hnt : u128.NUM_TIERS.val = 16 := by unfold u128.NUM_TIERS; simp
    have h0 := a_post 0 (by omega)
    have hi : i.val = 240 := by
      rw [i_post]
      simpa [Bijou.Family.offset_one, show Bijou.bijou128.threshold = 240 from rfl] using h0
    have hv : 240 ≤ v.val := by scalar_tac
    have hhi : v.val < 2 ^ 128 := by simpa using v.hBounds
    have hlog : Nat.log 2 v.val < 128 := Nat.log_lt_of_lt_pow (by omega) hhi
    have hlz : i1.val = 127 - Nat.log 2 v.val := by
      rw [i1_post]; exact leading_zeros_val (by omega)
    have htier : tier.val = Nat.log 2 v.val / 8 + 1 := by scalar_tac
    have hi6 : i6.val = Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by
      have := a_post i5.val (by omega)
      rw [i6_post, ← getElem!_pos, this]
      congr 1; omega
    obtain ⟨ht1, ht8⟩ := tierOf_bounds hv hhi
    split
    · rename_i hlt
      have hlt' : v.val < Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : i5.val = Bijou.bijou128.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_left hlt']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou128.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou128.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou128.offset i5.val := by
          have := a1_post i5.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (16 - Bijou.bijou128.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (16 - Bijou.bijou128.tierOf v.val)) =
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) < 2 ^ 128 := by
            calc _ < 256 ^ Bijou.bijou128.tierOf v.val * 256 ^ (16 - Bijou.bijou128.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 128 := by rw [← pow_add, show Bijou.bijou128.tierOf v.val +
                  (16 - Bijou.bijou128.tierOf v.val) = 16 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U128.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 240 := by
          rw [i7_post]; unfold u128.TAG_THRESHOLD; simp
        have htag : tag.val = 239 + Bijou.bijou128.tierOf v.val := by
          scalar_tac
        have hlen : i31.val = Bijou.bijou128.tierOf v.val + 1 := by
          have hi22 : i30.val = Bijou.bijou128.tierOf v.val + 1 := by rw [i30_post, ht]
          rw [i31_post]
          scalar_tac
        have hpb : pb.val.length = 16 := by simp
        have hnlt : ¬ v.val < Bijou.bijou128.threshold := by
          rw [show Bijou.bijou128.threshold = 240 from rfl]; omega
        refine ⟨by simp [hlen, Bijou.Family.encodedLen, hnlt], ?_⟩
        rw [encode_multi hv]
        simp only [Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post, i18_post, i19_post, i20_post, i21_post, i22_post, i23_post, i24_post, i25_post, i26_post, i27_post, i28_post, i29_post,
          sixteen_elems pb.val hpb, hlen, List.take_succ_cons, List.map_cons, htag,
          show 239 + Bijou.bijou128.tierOf v.val = 240 + (Bijou.bijou128.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl
    · rename_i hge
      have hge' : ¬ v.val < Bijou.bijou128.offset (Nat.log 2 v.val / 8 + 1) := by scalar_tac
      have ht : tier.val = Bijou.bijou128.tierOf v.val := by
        rw [tierOf_dispatch hv hhi, ite_eq_right hge']; omega
      step*
      · have hi10 : i10.val = Bijou.bijou128.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        rw [hi10, ht]
        exact Bijou.bijou128.offset_tierOf_le v.val
      · have hi10 : i10.val = Bijou.bijou128.offset tier.val := by
          have := a1_post tier.val (by omega)
          rw [i10_post, ← getElem!_pos, this]
        have hy := payload_lt hv hhi
        have hi11 : i11.val = v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val) := by
          rw [i11_post, hi10, ht]
        have hi13 : i13.val = 8 * (16 - Bijou.bijou128.tierOf v.val) := by
          rw [i13_post, i12_post, hnt, ht]
        have hpay : payload.bv.toNat =
            (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
          have hp2 : (2 : ℕ) ^ (8 * (16 - Bijou.bijou128.tierOf v.val)) =
              256 ^ (16 - Bijou.bijou128.tierOf v.val) := by
            rw [pow_mul]; norm_num
          have hfit : (v.val - Bijou.bijou128.offset (Bijou.bijou128.tierOf v.val)) *
              256 ^ (16 - Bijou.bijou128.tierOf v.val) < 2 ^ 128 := by
            calc _ < 256 ^ Bijou.bijou128.tierOf v.val * 256 ^ (16 - Bijou.bijou128.tierOf v.val) :=
                  Nat.mul_lt_mul_of_pos_right hy (by positivity)
              _ = 2 ^ 128 := by rw [← pow_add, show Bijou.bijou128.tierOf v.val +
                  (16 - Bijou.bijou128.tierOf v.val) = 16 by omega]; norm_num
          change payload.val = _
          rw [payload_post, hi11, hi13, Nat.shiftLeft_eq, hp2, U128.size_eq]
          exact Nat.mod_eq_of_lt (by simpa using hfit)
        have hi7 : i7.val = 240 := by
          rw [i7_post]; unfold u128.TAG_THRESHOLD; simp
        have htag : tag.val = 239 + Bijou.bijou128.tierOf v.val := by
          scalar_tac
        have hlen : i31.val = Bijou.bijou128.tierOf v.val + 1 := by
          have hi22 : i30.val = Bijou.bijou128.tierOf v.val + 1 := by rw [i30_post, ht]
          rw [i31_post]
          scalar_tac
        have hpb : pb.val.length = 16 := by simp
        have hnlt : ¬ v.val < Bijou.bijou128.threshold := by
          rw [show Bijou.bijou128.threshold = 240 from rfl]; omega
        refine ⟨by simp [hlen, Bijou.Family.encodedLen, hnlt], ?_⟩
        rw [encode_multi hv]
        simp only [Array.make, Array.from_val]
        rw [i14_post, i15_post, i16_post, i17_post, i18_post, i19_post, i20_post, i21_post, i22_post, i23_post, i24_post, i25_post, i26_post, i27_post, i28_post, i29_post,
          sixteen_elems pb.val hpb, hlen, List.take_succ_cons, List.map_cons, htag,
          show 239 + Bijou.bijou128.tierOf v.val = 240 + (Bijou.bijou128.tierOf v.val - 1) by omega,
          ← payload_bytes payload.bv _ _ ht8 hy hpay, pb_post]
        simp [List.map_take]
        rfl


/-- What callers see: `encoded_bytes(v).as_slice()` is exactly the model's
encoding of `v`, the same bytes `encode` appends. -/
theorem encoded_bytes_as_slice (v : Std.U128) :
    u128.encoded_bytes v ⦃ e => u128.EncodedU128.as_slice e ⦃ s =>
      s.val.map (·.val) = Bijou.bijou128.encode v.val ⦄ ⦄ := by
  apply WP.spec_mono (encoded_bytes_spec v)
  rintro e ⟨hl, hb⟩
  have hle : e.len.val ≤ 17 := by
    rw [hl]; exact Bijou.bijou128.encodedLen_le v.val
  apply WP.spec_mono (as_slice_spec e hle)
  intro s hs
  rw [hs, hb]

/-- The Rust decode result, read as the model's. -/
def toModel : core.result.Result (Std.U128 × Std.Usize) u128.DecodeError →
    Except Bijou.DecodeError (ℕ × ℕ)
  | .Ok (v, n) => .ok (v.val, n.val)
  | .Err .BufferTooShort => .error .bufferTooShort
  | .Err .Overflow => .error .overflow

theorem model_decode_multi (tag : ℕ) (rest : List ℕ) (h1 : 240 ≤ tag) :
    Bijou.bijou128.decode (tag :: rest) =
      if tag - 239 ≤ rest.length then
        if Bijou.bijou128.offset (tag - 239) + Bijou.fromBe (rest.take (tag - 239)) < 2 ^ 128 then
          .ok (Bijou.bijou128.offset (tag - 239) + Bijou.fromBe (rest.take (tag - 239)), tag - 238)
        else .error .overflow
      else .error .bufferTooShort := by
  have hthr : Bijou.bijou128.threshold = 240 := rfl
  have hpow : (256 : ℕ) ^ Bijou.bijou128.tiers = 2 ^ 128 := by norm_num [Bijou.bijou128]
  simp only [Bijou.Family.decode, hthr, show ¬ tag < 240 by omega, ite_false, hpow,
    show tag - 240 + 1 = tag - 239 by omega, show tag - 240 + 2 = tag - 238 by omega]

-- Generated by scripts/gen_widths.py from its `decode` template, for every
-- width (one arm per tier). Edit the template, not this proof.
-- 16 tiers: twice the bytes (and `decode` arms) of `u64`, so these two
-- whole-function proofs need a larger heartbeat budget than the default.
set_option maxHeartbeats 1000000 in
theorem decode_spec (buf : Slice Std.U8) :
    u128.decode buf ⦃ r => toModel r = Bijou.bijou128.decode (buf.val.map (·.val)) ⦄ := by
  unfold u128.decode
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
    change (if tag < u128.TAG_THRESHOLD then _ else _) ⦃ r =>
      toModel r = Bijou.bijou128.decode (buf.val.map (·.val)) ⦄
    split
    · step*
      have hlt' : tag.val < 240 := by
        have : u128.TAG_THRESHOLD.val = 240 := by unfold u128.TAG_THRESHOLD; rfl
        scalar_tac
      simp [o_post, toModel, Bijou.Family.decode, show Bijou.bijou128.threshold = 240 from rfl,
        hlt', i_post]
    · rename_i hge
      split
      · step*
        · rename_i hn
          rw [hn] at o1_post
          simp only at o1_post
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide)]
          have htv : (↑(240#8#uscalar : Std.U8) : ℕ) = 240 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(240#8#uscalar : Std.U8) : ℕ) = 240 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 1 := by
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
            have hz := fromBe_zeros 15 [a1.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(241#8#uscalar : Std.U8) : ℕ) = 241 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(241#8#uscalar : Std.U8) : ℕ) = 241 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 2 := by
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
            have hz := fromBe_zeros 14 [a1.val, b.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(242#8#uscalar : Std.U8) : ℕ) = 242 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(242#8#uscalar : Std.U8) : ℕ) = 242 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 3 := by
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
            have hz := fromBe_zeros 13 [a1.val, b.val, c.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(243#8#uscalar : Std.U8) : ℕ) = 243 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(243#8#uscalar : Std.U8) : ℕ) = 243 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 4 := by
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
            have hz := fromBe_zeros 12 [a1.val, b.val, c.val, d.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(244#8#uscalar : Std.U8) : ℕ) = 244 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(244#8#uscalar : Std.U8) : ℕ) = 244 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 5 := by
            have := a2_post 5 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 5).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 5 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post]
            have hz := fromBe_zeros 11 [a1.val, b.val, c.val, d.val, e.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(245#8#uscalar : Std.U8) : ℕ) = 245 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(245#8#uscalar : Std.U8) : ℕ) = 245 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 6 := by
            have := a2_post 6 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 6).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 6 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post]
            have hz := fromBe_zeros 10 [a1.val, b.val, c.val, d.val, e.val, f.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(246#8#uscalar : Std.U8) : ℕ) = 246 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(246#8#uscalar : Std.U8) : ℕ) = 246 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 7 := by
            have := a2_post 7 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 7).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 7 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post]
            have hz := fromBe_zeros 9 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(247#8#uscalar : Std.U8) : ℕ) = 247 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(247#8#uscalar : Std.U8) : ℕ) = 247 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 8 := by
            have := a2_post 8 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 8).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 8 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post]
            have hz := fromBe_zeros 8 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(248#8#uscalar : Std.U8) : ℕ) = 248 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(248#8#uscalar : Std.U8) : ℕ) = 248 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 9 := by
            have := a2_post 9 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 9).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 9 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post]
            have hz := fromBe_zeros 7 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(249#8#uscalar : Std.U8) : ℕ) = 249 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(249#8#uscalar : Std.U8) : ℕ) = 249 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 10 := by
            have := a2_post 10 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 10).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 10 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post]
            have hz := fromBe_zeros 6 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val, j.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(250#8#uscalar : Std.U8) : ℕ) = 250 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(250#8#uscalar : Std.U8) : ℕ) = 250 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 11 := by
            have := a2_post 11 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 11).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j, k] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 11 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post, k_post]
            have hz := fromBe_zeros 5 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val, j.val, k.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(251#8#uscalar : Std.U8) : ℕ) = 251 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(251#8#uscalar : Std.U8) : ℕ) = 251 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 12 := by
            have := a2_post 12 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 12).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j, k, l] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 12 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post, k_post, l_post]
            have hz := fromBe_zeros 4 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val, j.val, k.val, l.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have htv : (↑(252#8#uscalar : Std.U8) : ℕ) = 252 := rfl
          simp [toModel, o1_post, htv]
        all_goals
          rw [‹o1 = some a›] at o1_post
          simp only at o1_post
          obtain ⟨hlen, ha⟩ := o1_post
          have htv : (↑(252#8#uscalar : Std.U8) : ℕ) = 252 := rfl
          have hoff : offset.val = Bijou.bijou128.offset 13 := by
            have := a2_post 13 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 13).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j, k, l, m] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 13 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post, k_post, l_post, m_post]
            have hz := fromBe_zeros 3 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val, j.val, k.val, l.val, m.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have hoff : offset.val = Bijou.bijou128.offset 14 := by
            have := a2_post 14 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 14).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j, k, l, m, n] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 14 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post, k_post, l_post, m_post, n_post]
            have hz := fromBe_zeros 2 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val, j.val, k.val, l.val, m.val, n.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
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
          have hoff : offset.val = Bijou.bijou128.offset 15 := by
            have := a2_post 15 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 15).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j, k, l, m, n, o2] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 15 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post, k_post, l_post, m_post, n_post, o2_post]
            have hz := fromBe_zeros 1 [a1.val, b.val, c.val, d.val, e.val, f.val, g.val, h.val, i.val, j.val, k.val, l.val, m.val, n.val, o2.val]
            simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz
            rw [← ha, hlist]
            simpa [Array.make, Array.from_val, Function.comp] using hz
          rw [o_post, List.map_cons, model_decode_multi _ _ (by decide), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
          first
          | rw [‹o3 = none›] at o3_post
            simp only at o3_post
            rw [ite_eq_right (by omega)]
            rfl
          | rw [‹o3 = some value›] at o3_post
            obtain ⟨hle, hval, _⟩ := o3_post
            rw [ite_eq_left (by omega)]
            simp [toModel, hval]
      · rename_i x240 x241 x242 x243 x244 x245 x246 x247 x248 x249 x250 x251 x252 x253 x254
        have htt : u128.TAG_THRESHOLD.val = 240 := by unfold u128.TAG_THRESHOLD; rfl
        have hn240 : tag.val ≠ 240 := fun hv =>
          x240 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn241 : tag.val ≠ 241 := fun hv =>
          x241 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn242 : tag.val ≠ 242 := fun hv =>
          x242 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn243 : tag.val ≠ 243 := fun hv =>
          x243 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn244 : tag.val ≠ 244 := fun hv =>
          x244 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn245 : tag.val ≠ 245 := fun hv =>
          x245 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn246 : tag.val ≠ 246 := fun hv =>
          x246 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn247 : tag.val ≠ 247 := fun hv =>
          x247 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn248 : tag.val ≠ 248 := fun hv =>
          x248 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn249 : tag.val ≠ 249 := fun hv =>
          x249 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn250 : tag.val ≠ 250 := fun hv =>
          x250 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn251 : tag.val ≠ 251 := fun hv =>
          x251 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn252 : tag.val ≠ 252 := fun hv =>
          x252 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn253 : tag.val ≠ 253 := fun hv =>
          x253 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hn254 : tag.val ≠ 254 := fun hv =>
          x254 (UScalar.eq_of_val_eq (by rw [hv]; rfl))
        have hge' : 240 ≤ tag.val := by scalar_tac
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
          have hoff : offset.val = Bijou.bijou128.offset 16 := by
            have := a2_post 16 (by omega)
            rw [offset_post, ← getElem!_pos, this]
          have hpay : payload.val = Bijou.fromBe ((rest.val.take 16).map (·.val)) := by
            have hb := congrArg BitVec.toNat payload_post
            rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb
            change payload.val = _ at hb
            rw [hb]
            have hlist : a.val = [a1, b, c, d, e, f, g, h, i, j, k, l, m, n, o2, p1] := by
              apply List.ext_getElem
              · simp
              · intro idx hidx _
                have hj : idx < 16 := by have := a.property; simp_all
                interval_cases idx <;>
                simp [a1_post, b_post, c_post, d_post, e_post, f_post, g_post, h_post, i_post, j_post, k_post, l_post, m_post, n_post, o2_post, p1_post]
            rw [← ha, hlist]
            simp [Array.make, Array.from_val, Function.comp]
          rw [o_post, List.map_cons, model_decode_multi _ _ (by omega), htv,
            ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]
          have hmax : U128.max = 2 ^ 128 - 1 := by rw [U128.max_eq]; norm_num
          first
          | rw [‹o3 = none›] at o3_post
            simp only at o3_post
            rw [ite_eq_right (by omega)]
            rfl
          | rw [‹o3 = some value›] at o3_post
            obtain ⟨hle, hval, _⟩ := o3_post
            rw [ite_eq_left (by omega)]
            simp [toModel, hval]

end BijouAeneas.U128
