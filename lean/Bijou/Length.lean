module

public import Bijou.Signed

@[expose] public section

/-!
# Encoded length as a function of the value

The SPECs' tier tables and design goals make claims about *which* values get
*how many* bytes. `Instances.lean` and `SignedInstances.lean` check those
tables at their row boundaries; these theorems state them in general.

- `threshold_add_tiers` / `threshold_widest` — every tag byte is used, so the
  threshold is the widest the tier count allows (SPEC "Width Variants").
- `encodedLen_mono` — larger values never encode shorter.
- `encodedLen_eq_one_iff` — one byte exactly for values below the threshold
  (SPEC: "values 0–247 are encoded as a single byte").
- `encodedLen_eq_succ_iff` — the tier table: `t + 1` bytes exactly for
  `[offset t, offset (t + 1))`.
- `encodedLenS_mono` — signed: a strictly larger magnitude never encodes
  shorter, whatever the signs (bijou64s: "encoded length grows with
  magnitude regardless of sign").
- `encodedLenS_eq_one_iff` — signed: one byte exactly for
  `[-⌊T/2⌋, ⌈T/2⌉)` around zero, `T` the threshold (bijou64s: "one byte
  for the 248 values nearest zero", `-124 ..= 123`).
- `encodedLenS_eq_succ_iff` — the signed tier table, through `zigzag`.
-/

namespace Bijou

namespace Family

variable (F : Family)

/-- Every tag byte is used: `threshold` single-byte literals, then one tag per
multi-byte tier. -/
theorem threshold_add_tiers : F.threshold + F.tiers = 256 := by
  have := F.tiers_lt
  simp only [threshold]
  omega

/-- The threshold is the widest the tier count allows (SPEC "Width Variants":
"each variant uses the widest tag threshold its tier count allows"): any larger
threshold would leave fewer than `tiers` tag bytes for the multi-byte tiers. -/
theorem threshold_widest {T : Nat} (h : T + F.tiers ≤ 256) : T ≤ F.threshold := by
  have := F.threshold_add_tiers
  omega

/-- `tierOf` is monotone. -/
theorem tierOf_mono {v w : Nat} (hw : w < 256 ^ F.tiers) (h : v ≤ w) :
    F.tierOf v ≤ F.tierOf w := by
  have ha := F.offset_tierOf_le v
  have hb := F.lt_offset_tierOf_succ hw
  refine Nat.le_of_not_lt fun hlt => ?_
  have := F.offset_le_offset (show F.tierOf w + 1 ≤ F.tierOf v from hlt)
  omega

/-- Larger values never encode shorter. -/
theorem encodedLen_mono {v w : Nat} (hw : w < 256 ^ F.tiers) (h : v ≤ w) :
    F.encodedLen v ≤ F.encodedLen w := by
  have := F.tierOf_mono hw h
  simp only [encodedLen]
  split <;> split <;> omega

/-- One byte exactly for the values below the threshold. -/
theorem encodedLen_eq_one_iff (v : Nat) : F.encodedLen v = 1 ↔ v < F.threshold := by
  simp only [encodedLen]
  split
  · simp [*]
  · rename_i h
    have := F.tierOf_pos (Nat.le_of_not_lt h)
    constructor <;> intro <;> omega

/-- The tier table: for `t ≥ 1`, `t + 1` bytes exactly for the values in
`[offset t, offset (t + 1))`. -/
theorem encodedLen_eq_succ_iff {v t : Nat} (hv : v < 256 ^ F.tiers) (ht : 1 ≤ t) :
    F.encodedLen v = t + 1 ↔ F.offset t ≤ v ∧ v < F.offset (t + 1) := by
  have hthr : F.threshold ≤ F.offset t := by
    rw [← F.offset_one]; exact F.offset_le_offset ht
  constructor
  · intro h
    simp only [encodedLen] at h
    split at h
    · omega
    · have ht' : F.tierOf v = t := by omega
      exact ⟨ht' ▸ F.offset_tierOf_le v, ht' ▸ F.lt_offset_tierOf_succ hv⟩
  · rintro ⟨h1, h2⟩
    simp only [encodedLen, show ¬ v < F.threshold by omega, ite_false, F.tierOf_eq hv h1 h2]

/-- A strictly larger magnitude has a strictly larger zigzag image, whatever
the signs. -/
theorem _root_.Bijou.zigzag_lt_of_natAbs_lt {i j : Int} (h : i.natAbs < j.natAbs) :
    zigzag i < zigzag j := by
  cases i <;> cases j <;> simp only [zigzag, Int.natAbs] at h ⊢ <;> omega

/-- Signed: a strictly larger magnitude never encodes shorter, whatever the
signs. (At *equal* magnitude the negative value can be one byte shorter:
`zigzag (-k) = 2k - 1 < 2k = zigzag k`.) -/
theorem encodedLenS_mono {i j : Int}
    (hj₁ : -(F.half : Int) ≤ j) (hj₂ : j < (F.half : Int)) (h : i.natAbs < j.natAbs) :
    F.encodedLenS i ≤ F.encodedLenS j :=
  F.encodedLen_mono (F.zigzag_lt_pow hj₁ hj₂) (Nat.le_of_lt (zigzag_lt_of_natAbs_lt h))

/-- Signed: one byte exactly for `[-⌊T/2⌋, ⌈T/2⌉)`, `T = threshold`. For
bijou64s (`T = 248`) that is `-124 ..= 123`, the 248 values nearest zero. -/
theorem encodedLenS_eq_one_iff (i : Int) :
    F.encodedLenS i = 1 ↔
      -((F.threshold / 2 : Nat) : Int) ≤ i ∧ i < ((F.threshold + 1) / 2 : Nat) := by
  rw [encodedLenS, encodedLen_eq_one_iff]
  cases i <;> simp only [zigzag, Int.ofNat_eq_natCast, Int.negSucc_eq] <;> omega

/-- The signed tier table: for `t ≥ 1`, `t + 1` bytes exactly for the values
whose zigzag image lies in `[offset t, offset (t + 1))`. -/
theorem encodedLenS_eq_succ_iff {i : Int} {t : Nat}
    (hi₁ : -(F.half : Int) ≤ i) (hi₂ : i < (F.half : Int)) (ht : 1 ≤ t) :
    F.encodedLenS i = t + 1 ↔ F.offset t ≤ zigzag i ∧ zigzag i < F.offset (t + 1) :=
  F.encodedLen_eq_succ_iff (F.zigzag_lt_pow hi₁ hi₂) ht

end Family

end Bijou
