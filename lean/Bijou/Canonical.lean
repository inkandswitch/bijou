import Bijou.RoundTrip

/-!
# Canonicality, by construction

The SPEC's defining claim: every value has exactly one encoding, and
every byte string the decoder accepts *is* the canonical encoding of
the value it returns. There are no overlong encodings to reject — the
per-tier offset arithmetic makes them unrepresentable.

- `decode_ok`: anything the decoder accepts is, byte for byte, the
  output of `encode` (and the value is in range).
- `encode_injective`: distinct values have distinct encodings.
- `decode_canonical`: two fully-consumed buffers decoding to the same
  value are equal.
-/

namespace Bijou.Family

variable (F : Family)

/-- If `decode bs = ok (v, n)` then the `n` bytes consumed are exactly
`encode v`: the decoder only ever accepts canonical encodings. -/
theorem decode_ok {bs : List Nat} {v n : Nat}
    (hbytes : ∀ b ∈ bs, b < 256)
    (h : F.decode bs = .ok (v, n)) :
    v < 256 ^ F.tiers ∧ n = F.encodedLen v ∧ bs.take n = F.encode v := by
  match bs with
  | [] => simp [decode] at h
  | tag :: rest =>
    simp only [decode] at h
    by_cases htag : tag < F.threshold
    · rw [if_pos htag] at h
      simp only [Except.ok.injEq, Prod.mk.injEq] at h
      have hv : tag = v := h.1
      have hn : 1 = n := h.2
      subst hv
      subst hn
      have h256 := F.le_pow_tiers
      have hthr := F.threshold_lt
      refine ⟨by omega, ?_, ?_⟩
      · simp [encodedLen, htag]
      · simp [encode, htag]
    · rw [if_neg htag] at h
      by_cases hlen : tag - F.threshold + 1 ≤ rest.length
      · rw [if_pos hlen] at h
        by_cases hov :
            F.offset (tag - F.threshold + 1)
                + fromBe (rest.take (tag - F.threshold + 1))
              < 256 ^ F.tiers
        · rw [if_pos hov] at h
          simp only [Except.ok.injEq, Prod.mk.injEq] at h
          have hv := h.1
          have hn := h.2
          subst hv
          subst hn
          -- Abbreviations (as facts, since `set` is unavailable).
          have htlen : (rest.take (tag - F.threshold + 1)).length
              = tag - F.threshold + 1 := by
            simp only [List.length_take]
            omega
          have hpb : ∀ b ∈ rest.take (tag - F.threshold + 1), b < 256 := fun b hb =>
            hbytes b (List.mem_cons_of_mem _ (mem_of_mem_take hb))
          have hpay : fromBe (rest.take (tag - F.threshold + 1))
              < 256 ^ (tag - F.threshold + 1) := by
            have h' := fromBe_lt hpb
            rwa [htlen] at h'
          -- The value lands inside tier `tag - threshold + 1`.
          have hub : F.offset (tag - F.threshold + 1)
                + fromBe (rest.take (tag - F.threshold + 1))
              < F.offset (tag - F.threshold + 1 + 1) := by
            rw [F.offset_succ (tag - F.threshold + 1),
              F.capacity_eq_pow (show 0 < tag - F.threshold + 1 by omega)]
            omega
          -- The accepted tier is the tier of the value.
          have htier : F.tierOf
              (F.offset (tag - F.threshold + 1)
                + fromBe (rest.take (tag - F.threshold + 1)))
              = tag - F.threshold + 1 :=
            F.tierOf_eq hov (Nat.le_add_right _ _) hub
          -- The value is multi-byte: at or above the threshold.
          have hge : F.threshold
              ≤ F.offset (tag - F.threshold + 1)
                + fromBe (rest.take (tag - F.threshold + 1)) := by
            have h1 : F.offset 1 ≤ F.offset (tag - F.threshold + 1) :=
              F.offset_le_offset (by omega)
            rw [F.offset_one] at h1
            omega
          have hnlt : ¬(F.offset (tag - F.threshold + 1)
                + fromBe (rest.take (tag - F.threshold + 1))
              < F.threshold) := by omega
          refine ⟨hov, ?_, ?_⟩
          · simp only [encodedLen, if_neg hnlt, htier]
          · simp only [encode, if_neg hnlt, htier]
            have htag' : tag - F.threshold + 2 = (tag - F.threshold + 1) + 1 := by omega
            rw [htag', List.take_succ_cons]
            have hhead : F.threshold + (tag - F.threshold + 1 - 1) = tag := by omega
            have hsub : F.offset (tag - F.threshold + 1)
                + fromBe (rest.take (tag - F.threshold + 1))
                - F.offset (tag - F.threshold + 1)
                = fromBe (rest.take (tag - F.threshold + 1)) := by omega
            rw [hhead, hsub]
            have hbe := beBytes_fromBe hpb
            rw [htlen] at hbe
            rw [hbe]
        · rw [if_neg hov] at h
          simp at h
      · rw [if_neg hlen] at h
        simp at h

/-- Distinct values never share an encoding. -/
theorem encode_injective {v₁ v₂ : Nat}
    (h₁ : v₁ < 256 ^ F.tiers) (h₂ : v₂ < 256 ^ F.tiers)
    (h : F.encode v₁ = F.encode v₂) : v₁ = v₂ := by
  have d₁ := F.decode_encode' h₁
  have d₂ := F.decode_encode' h₂
  rw [h, d₂] at d₁
  simp only [Except.ok.injEq, Prod.mk.injEq] at d₁
  exact d₁.1.symm

/-- Two fully-consumed valid buffers decoding to the same value are
identical: there is exactly one byte representation per value. -/
theorem decode_canonical {bs₁ bs₂ : List Nat} {v : Nat}
    (hb₁ : ∀ b ∈ bs₁, b < 256) (hb₂ : ∀ b ∈ bs₂, b < 256)
    (h₁ : F.decode bs₁ = .ok (v, bs₁.length))
    (h₂ : F.decode bs₂ = .ok (v, bs₂.length)) :
    bs₁ = bs₂ := by
  have c₁ := (F.decode_ok hb₁ h₁).2.2
  have c₂ := (F.decode_ok hb₂ h₂).2.2
  rw [List.take_length] at c₁ c₂
  rw [c₁, c₂]

/-- Overflow is signalled only at the maximal tag `255` (`0xFF`) — the
top tier. Lower tiers cannot overflow, because their entire value range
sits below `256 ^ tiers`. This is the SPEC's "Minimal Decoder
Obligations": the overflow check is needed only for the widest tier. -/
theorem decode_overflow_max_tag {tag : Nat} {rest : List Nat}
    (hbytes : ∀ b ∈ tag :: rest, b < 256)
    (h : F.decode (tag :: rest) = .error .overflow) :
    tag = 255 := by
  have htag256 : tag < 256 := hbytes tag (List.mem_cons_self ..)
  have hthr : F.threshold = 256 - F.tiers := rfl
  have htiers := F.tiers_pos
  have htierslt := F.tiers_lt
  simp only [decode] at h
  by_cases htag : tag < F.threshold
  · rw [if_pos htag] at h; simp at h
  · rw [if_neg htag] at h
    by_cases hlen : tag - F.threshold + 1 ≤ rest.length
    · rw [if_pos hlen] at h
      by_cases hov :
          F.offset (tag - F.threshold + 1) + fromBe (rest.take (tag - F.threshold + 1))
            < 256 ^ F.tiers
      · rw [if_pos hov] at h; simp at h
      · -- Overflow branch: the decoded value is ≥ 256 ^ tiers.
        have htpos : 0 < tag - F.threshold + 1 := by omega
        have htle : tag - F.threshold + 1 ≤ F.tiers := by omega
        have hnov : 256 ^ F.tiers
            ≤ F.offset (tag - F.threshold + 1) + fromBe (rest.take (tag - F.threshold + 1)) :=
          Nat.not_lt.mp hov
        have htlen : (rest.take (tag - F.threshold + 1)).length = tag - F.threshold + 1 := by
          simp only [List.length_take]; omega
        have hpb : ∀ b ∈ rest.take (tag - F.threshold + 1), b < 256 := fun b hb =>
          hbytes b (List.mem_cons_of_mem _ (mem_of_mem_take hb))
        have hpay : fromBe (rest.take (tag - F.threshold + 1)) < 256 ^ (tag - F.threshold + 1) := by
          have := fromBe_lt hpb
          rwa [htlen] at this
        -- A tier strictly below the top fits entirely under `256 ^ tiers`,
        -- so it cannot overflow. Hence the tier must be the top one.
        have hfull : ¬ tag - F.threshold + 1 < F.tiers := by
          intro htlt
          have hsucc : F.offset (tag - F.threshold + 1 + 1)
              = F.offset (tag - F.threshold + 1) + 256 ^ (tag - F.threshold + 1) := by
            rw [F.offset_succ, F.capacity_eq_pow htpos]
          have hmono : F.offset (tag - F.threshold + 1 + 1) ≤ F.offset F.tiers :=
            F.offset_le_offset (by omega)
          have htop : F.offset F.tiers < 256 ^ F.tiers := F.offset_lt_pow F.tiers_pos
          omega
        omega
    · rw [if_neg hlen] at h; simp at h

/-- The bijection, packaged. On the value domain `[0, 256 ^ tiers)`:
`encode` is injective; `decode` recovers the value it encoded
(round-trip); and `decode` accepts nothing but canonical encodings.
Together these state that `encode` is a bijection from the value domain
onto the set of decodable byte strings. (Stated elementarily so the
development stays Mathlib-free.) -/
theorem encode_bijection :
    (∀ {v₁ v₂ : Nat}, v₁ < 256 ^ F.tiers → v₂ < 256 ^ F.tiers →
        F.encode v₁ = F.encode v₂ → v₁ = v₂)
    ∧ (∀ {v : Nat}, v < 256 ^ F.tiers →
        F.decode (F.encode v) = .ok (v, (F.encode v).length))
    ∧ (∀ {bs : List Nat} {v n : Nat}, (∀ b ∈ bs, b < 256) → F.decode bs = .ok (v, n) →
        v < 256 ^ F.tiers ∧ bs.take n = F.encode v) :=
  ⟨fun h₁ h₂ h => F.encode_injective h₁ h₂ h,
   fun hv => F.decode_encode' hv,
   fun hb h => ⟨(F.decode_ok hb h).1, (F.decode_ok hb h).2.2⟩⟩

end Bijou.Family
