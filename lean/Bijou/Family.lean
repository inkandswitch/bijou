/-!
# The bijou format family

A bijou format is determined by a single parameter: the number of
multi-byte tiers. The tag-byte threshold is derived (`256 - tiers`),
so the multi-byte tags exactly fill the top of the byte range, and the
value domain is `[0, 256 ^ tiers)`.

| Variant  | tiers | threshold | domain      |
|----------|-------|-----------|-------------|
| bijou32  | 4     | 252       | `[0, 2^32)` |
| bijou64  | 8     | 248       | `[0, 2^64)` |
| bijou128 | 16    | 240       | `[0, 2^128)`|

Each tier `t ≥ 1` carries `256 ^ t` values starting at `offset t`, and
tier 0 (single byte) carries values `0 .. threshold - 1`. The offsets
tile the naturals contiguously, which is what makes the encoding
bijective: there is exactly one tier for every value.
-/

namespace Bijou

/-- A bijou format family, parametrized by the number of multi-byte
tiers (4 for bijou32, 8 for bijou64, 16 for bijou128). -/
structure Family where
  /-- Number of multi-byte tiers. -/
  tiers : Nat
  tiers_pos : 0 < tiers
  tiers_lt : tiers < 256

namespace Family

variable (F : Family)

/-- Tag-byte threshold: values below this encode as a single byte, and
tags `threshold .. 255` introduce tiers `1 .. tiers`. -/
def threshold : Nat := 256 - F.tiers

theorem threshold_pos : 0 < F.threshold := by
  have := F.tiers_lt
  simp only [threshold]
  omega

theorem threshold_lt : F.threshold < 256 := by
  have := F.tiers_pos
  simp only [threshold]
  omega

/-- Number of values representable by tier `t` alone. -/
def capacity : Nat → Nat
  | 0 => F.threshold
  | t + 1 => 256 ^ (t + 1)

theorem capacity_pos (t : Nat) : 0 < F.capacity t := by
  cases t with
  | zero => exact F.threshold_pos
  | succ t => exact Nat.pow_pos (by omega)

theorem capacity_eq_pow {t : Nat} (h : 0 < t) : F.capacity t = 256 ^ t := by
  cases t with
  | zero => omega
  | succ t => rfl

/-- First value requiring tier `t`: the cumulative capacity of all
previous tiers. This is the SPEC's `OFFSET` table. -/
def offset : Nat → Nat
  | 0 => 0
  | t + 1 => offset t + F.capacity t

@[simp]
theorem offset_zero : F.offset 0 = 0 := rfl

theorem offset_succ (t : Nat) : F.offset (t + 1) = F.offset t + F.capacity t := rfl

theorem offset_one : F.offset 1 = F.threshold := by
  simp [offset, capacity]

theorem offset_lt_succ (t : Nat) : F.offset t < F.offset (t + 1) := by
  have := F.capacity_pos t
  rw [offset_succ]
  omega

theorem offset_le_offset {s t : Nat} (h : s ≤ t) : F.offset s ≤ F.offset t := by
  induction h with
  | refl => exact Nat.le_refl _
  | step _ ih => exact Nat.le_trans ih (Nat.le_of_lt (F.offset_lt_succ _))

theorem offset_lt_offset {s t : Nat} (h : s < t) : F.offset s < F.offset t :=
  Nat.lt_of_lt_of_le (F.offset_lt_succ s) (F.offset_le_offset h)

/-- Offsets stay strictly below the tier's own capacity ceiling, so
every tier has room: `offset t ≤ v < offset t + 256 ^ t` is satisfiable
for all `t ≥ 1`. -/
theorem offset_lt_pow : ∀ {t : Nat}, 0 < t → F.offset t < 256 ^ t := by
  intro t
  induction t with
  | zero => omega
  | succ t ih =>
    intro _
    cases Nat.eq_zero_or_pos t with
    | inl h0 =>
      subst h0
      have := F.threshold_lt
      rw [F.offset_one, Nat.pow_one]
      omega
    | inr hpos =>
      have ih' := ih hpos
      have hx : 0 < 256 ^ t := Nat.pow_pos (by omega)
      have hpow : 256 ^ (t + 1) = 256 ^ t * 256 := by rw [Nat.pow_succ]
      rw [F.offset_succ, F.capacity_eq_pow hpos]
      omega

/-- Tiers beyond `tiers` start past the value domain; the decoder's
overflow check rejects them with no special-casing. -/
theorem pow_le_offset {t : Nat} (h : F.tiers < t) : 256 ^ F.tiers ≤ F.offset t := by
  have h1 : 256 ^ F.tiers ≤ F.offset (F.tiers + 1) := by
    rw [F.offset_succ, F.capacity_eq_pow F.tiers_pos]
    omega
  exact Nat.le_trans h1 (F.offset_le_offset h)

theorem le_pow_tiers : 256 ≤ 256 ^ F.tiers := by
  have h := F.tiers_pos
  cases ht : F.tiers with
  | zero => omega
  | succ t =>
    have hx : 0 < 256 ^ t := Nat.pow_pos (by omega)
    have hpow : 256 ^ (t + 1) = 256 ^ t * 256 := by rw [Nat.pow_succ]
    omega

/-- Largest tier `s ≤ t` whose offset is at most `v`. -/
def tierSearch (v : Nat) : Nat → Nat
  | 0 => 0
  | t + 1 => if F.offset (t + 1) ≤ v then t + 1 else tierSearch v t

theorem tierSearch_le (v t : Nat) : F.tierSearch v t ≤ t := by
  induction t with
  | zero => exact Nat.le_refl _
  | succ t ih =>
    simp only [tierSearch]
    split
    · exact Nat.le_refl _
    · exact Nat.le_succ_of_le ih

theorem offset_tierSearch_le (v t : Nat) : F.offset (F.tierSearch v t) ≤ v := by
  induction t with
  | zero => simp [tierSearch]
  | succ t ih =>
    simp only [tierSearch]
    split
    · assumption
    · exact ih

theorem tierSearch_spec (v t : Nat) :
    v < F.offset (F.tierSearch v t + 1) ∨ F.tierSearch v t = t := by
  induction t with
  | zero => exact Or.inr rfl
  | succ t ih =>
    simp only [tierSearch]
    split
    · exact Or.inr rfl
    · rename_i hno
      cases ih with
      | inl h => exact Or.inl h
      | inr h => exact Or.inl (by rw [h]; omega)

theorem tierSearch_pos {v : Nat} (hv : F.threshold ≤ v) :
    ∀ {t : Nat}, 0 < t → 0 < F.tierSearch v t := by
  intro t
  induction t with
  | zero => omega
  | succ t ih =>
    intro _
    simp only [tierSearch]
    split
    · omega
    · rename_i hno
      cases Nat.eq_zero_or_pos t with
      | inl h0 =>
        subst h0
        rw [Nat.zero_add, F.offset_one] at hno
        omega
      | inr hpos => exact ih hpos

/-- The tier of `v`: for `v < 256 ^ tiers`, the unique `t` with
`offset t ≤ v < offset (t + 1)`. -/
def tierOf (v : Nat) : Nat := F.tierSearch v F.tiers

theorem tierOf_le (v : Nat) : F.tierOf v ≤ F.tiers :=
  F.tierSearch_le v F.tiers

theorem offset_tierOf_le (v : Nat) : F.offset (F.tierOf v) ≤ v :=
  F.offset_tierSearch_le v F.tiers

theorem lt_offset_tierOf_succ {v : Nat} (hv : v < 256 ^ F.tiers) :
    v < F.offset (F.tierOf v + 1) := by
  cases F.tierSearch_spec v F.tiers with
  | inl h => exact h
  | inr h =>
    have h1 : 256 ^ F.tiers ≤ F.offset (F.tiers + 1) := by
      rw [F.offset_succ, F.capacity_eq_pow F.tiers_pos]
      omega
    simp only [tierOf]
    rw [h]
    omega

theorem tierOf_pos {v : Nat} (hv : F.threshold ≤ v) : 0 < F.tierOf v :=
  F.tierSearch_pos hv F.tiers_pos

/-- Tier ranges are disjoint: any `t` whose range contains `v` is
*the* tier of `v`. -/
theorem tierOf_eq {v t : Nat} (hv : v < 256 ^ F.tiers)
    (h1 : F.offset t ≤ v) (h2 : v < F.offset (t + 1)) : F.tierOf v = t := by
  have ha := F.offset_tierOf_le v
  have hb := F.lt_offset_tierOf_succ hv
  cases Nat.lt_trichotomy (F.tierOf v) t with
  | inl hlt =>
    have := F.offset_le_offset (show F.tierOf v + 1 ≤ t from hlt)
    omega
  | inr h' =>
    cases h' with
    | inl heq => exact heq
    | inr hgt =>
      have := F.offset_le_offset (show t + 1 ≤ F.tierOf v from hgt)
      omega

end Family

end Bijou
