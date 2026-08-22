import Bijou.Spec

/-!
# Order preservation

Encodings sort in the same order as the values they represent:
lexicographic byte comparison equals numeric comparison. This is the
SPEC's "big-endian byte order" design goal — sorted storage and binary
search work directly on encoded bytes, without decoding.

The argument splits on tiers. Distinct tiers are separated by their
tag bytes (tier-0 values are below the threshold, and multi-byte tags
grow with the tier), while within a tier the big-endian payload
inherits numeric order (`beBytes_lex_beBytes`).
-/

namespace Bijou.Family

variable (F : Family)

theorem lex_encode_of_lt {v₁ v₂ : Nat} (h₂ : v₂ < 256 ^ F.tiers) (h : v₁ < v₂) :
    Lex (F.encode v₁) (F.encode v₂) := by
  by_cases ha : v₁ < F.threshold
  · by_cases hb : v₂ < F.threshold
    · simp only [encode, if_pos ha, if_pos hb]
      exact Lex.head h
    · have ht₂ := F.tierOf_pos (Nat.le_of_not_lt hb)
      simp only [encode, if_pos ha, if_neg hb]
      exact Lex.head (by omega)
  · have hb : ¬v₂ < F.threshold := by omega
    have h₁ : v₁ < 256 ^ F.tiers := by omega
    have ht₁ := F.tierOf_pos (Nat.le_of_not_lt ha)
    have ht₂ := F.tierOf_pos (Nat.le_of_not_lt hb)
    have ho₁l := F.offset_tierOf_le v₁
    have ho₁r := F.lt_offset_tierOf_succ h₁
    have ho₂l := F.offset_tierOf_le v₂
    have ho₂r := F.lt_offset_tierOf_succ h₂
    simp only [encode, if_neg ha, if_neg hb]
    cases Nat.lt_trichotomy (F.tierOf v₁) (F.tierOf v₂) with
    | inl hlt => exact Lex.head (by omega)
    | inr h' =>
      cases h' with
      | inl heq =>
        rw [heq] at ho₁l ho₁r ⊢
        rw [F.offset_succ, F.capacity_eq_pow ht₂] at ho₂r
        exact Lex.tail (beBytes_lex_beBytes (by omega) (by omega))
      | inr hgt =>
        have := F.offset_le_offset (show F.tierOf v₂ + 1 ≤ F.tierOf v₁ from hgt)
        omega

/-- Numeric order and lexicographic byte order coincide. -/
theorem encode_lex_iff {v₁ v₂ : Nat}
    (h₁ : v₁ < 256 ^ F.tiers) (h₂ : v₂ < 256 ^ F.tiers) :
    v₁ < v₂ ↔ Lex (F.encode v₁) (F.encode v₂) := by
  constructor
  · exact F.lex_encode_of_lt h₂
  · intro hlex
    cases Nat.lt_trichotomy v₁ v₂ with
    | inl h => exact h
    | inr h' =>
      cases h' with
      | inl heq =>
        subst heq
        exact absurd hlex (Lex.irrefl _)
      | inr hgt => exact absurd hlex (Lex.asymm (F.lex_encode_of_lt h₁ hgt))

/-- Any two in-range encodings are comparable: byte order is total, so
encoded values can be sorted directly on their bytes. -/
theorem encode_lex_trichotomy {v₁ v₂ : Nat}
    (h₁ : v₁ < 256 ^ F.tiers) (h₂ : v₂ < 256 ^ F.tiers) :
    Lex (F.encode v₁) (F.encode v₂)
      ∨ F.encode v₁ = F.encode v₂
      ∨ Lex (F.encode v₂) (F.encode v₁) := by
  rcases Nat.lt_trichotomy v₁ v₂ with h | h | h
  · exact Or.inl (F.lex_encode_of_lt h₂ h)
  · exact Or.inr (Or.inl (by rw [h]))
  · exact Or.inr (Or.inr (F.lex_encode_of_lt h₁ h))

end Bijou.Family
