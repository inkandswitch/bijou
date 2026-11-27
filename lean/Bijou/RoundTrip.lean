module

public import Bijou.Spec

@[expose] public section

/-!
# Round-trip

Decoding an encoding returns the original value and consumes exactly
`encodedLen v` bytes, regardless of trailing data.
-/

namespace Bijou.Family

variable (F : Family)

theorem decode_encode {v : Nat} (hv : v < 256 ^ F.tiers) (rest : List Nat) :
    F.decode (F.encode v ++ rest) = .ok (v, F.encodedLen v) := by
  by_cases h : v < F.threshold
  · simp [encode, decode, encodedLen, h]
  · have ht1 : 0 < F.tierOf v := F.tierOf_pos (by omega)
    have ho1 := F.offset_tierOf_le v
    have ho2 := F.lt_offset_tierOf_succ hv
    rw [F.offset_succ, F.capacity_eq_pow ht1] at ho2
    have htag : ¬F.threshold + (F.tierOf v - 1) < F.threshold := by omega
    have hT : F.threshold + (F.tierOf v - 1) - F.threshold + 1 = F.tierOf v := by omega
    have hT2 : F.threshold + (F.tierOf v - 1) - F.threshold + 2 = F.tierOf v + 1 := by
      omega
    simp only [encode, encodedLen, ite_eq_right h, List.cons_append, decode, ite_eq_right htag,
      hT, hT2]
    have hlen :
        F.tierOf v ≤ (beBytes (F.tierOf v) (v - F.offset (F.tierOf v)) ++ rest).length := by
      simp
    rw [ite_eq_left hlen]
    have htake :
        (beBytes (F.tierOf v) (v - F.offset (F.tierOf v)) ++ rest).take (F.tierOf v)
          = beBytes (F.tierOf v) (v - F.offset (F.tierOf v)) := by
      have h' := take_append_length (beBytes (F.tierOf v) (v - F.offset (F.tierOf v))) rest
      rwa [beBytes_length] at h'
    rw [htake, fromBe_beBytes (by omega)]
    have hval : F.offset (F.tierOf v) + (v - F.offset (F.tierOf v)) = v := by omega
    rw [hval, ite_eq_left hv]

/-- An encoding alone (no trailing bytes) decodes fully: the consumed
count is the entire buffer. -/
theorem decode_encode' {v : Nat} (hv : v < 256 ^ F.tiers) :
    F.decode (F.encode v) = .ok (v, (F.encode v).length) := by
  have h := F.decode_encode hv []
  rw [List.append_nil] at h
  rw [h, F.encode_length]

end Bijou.Family
