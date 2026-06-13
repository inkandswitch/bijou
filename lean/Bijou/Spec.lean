import Bijou.Bytes
import Bijou.Family

/-!
# Encoding and decoding

Direct transcriptions of the SPEC's Encoding and Decoding sections,
over `Nat` (values bounded by `256 ^ tiers`, bytes by `256`).

- `Family.encode`: tier 0 emits the value as a single byte; tier `t`
  emits tag `threshold + (t - 1)` followed by the `t`-byte big-endian
  payload `v - offset t`.
- `Family.decode`: reads the tag, takes `tag - threshold + 1` payload
  bytes, and returns `offset t + payload` plus the consumed length.
  The only error conditions are a short buffer and top-tier overflow,
  exactly as the SPEC's "Minimal Decoder Obligations" demands.
-/

namespace Bijou

/-- The only two decode failures a conforming decoder may signal.
There is no "non-canonical encoding" error: non-canonical encodings
are structurally impossible. -/
inductive DecodeError where
  | bufferTooShort
  | overflow
deriving DecidableEq, Repr

namespace Family

variable (F : Family)

/-- Encode `v < 256 ^ tiers` as a bijou byte string. -/
def encode (v : Nat) : List Nat :=
  if v < F.threshold then [v]
  else
    (F.threshold + (F.tierOf v - 1))
      :: beBytes (F.tierOf v) (v - F.offset (F.tierOf v))

/-- The number of bytes `encode v` produces. -/
def encodedLen (v : Nat) : Nat :=
  if v < F.threshold then 1 else F.tierOf v + 1

/-- Decode a value from the front of `bs`, returning it together with
the number of bytes consumed. -/
def decode (bs : List Nat) : Except DecodeError (Nat × Nat) :=
  match bs with
  | [] => .error .bufferTooShort
  | tag :: rest =>
    if tag < F.threshold then .ok (tag, 1)
    else if tag - F.threshold + 1 ≤ rest.length then
      if F.offset (tag - F.threshold + 1) + fromBe (rest.take (tag - F.threshold + 1))
          < 256 ^ F.tiers then
        .ok
          ( F.offset (tag - F.threshold + 1) + fromBe (rest.take (tag - F.threshold + 1))
          , tag - F.threshold + 2
          )
      else .error .overflow
    else .error .bufferTooShort

@[simp]
theorem encode_length (v : Nat) : (F.encode v).length = F.encodedLen v := by
  simp only [encode, encodedLen]
  split
  · rfl
  · simp

/-- The encoder emits genuine bytes. -/
theorem encode_bytes_lt {v : Nat} (hv : v < 256 ^ F.tiers) :
    ∀ b ∈ F.encode v, b < 256 := by
  intro b hb
  simp only [encode] at hb
  split at hb
  · have := F.threshold_lt
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hb
    omega
  · rename_i hge
    simp only [List.mem_cons] at hb
    cases hb with
    | inl h =>
      have ht := F.tierOf_le v
      have h1 := F.tiers_lt
      have h2 := F.tiers_pos
      subst h
      simp only [threshold]
      omega
    | inr h =>
      have h1 := F.offset_tierOf_le v
      have h2 := F.lt_offset_tierOf_succ hv
      have h3 : 0 < F.tierOf v := F.tierOf_pos (by omega)
      rw [F.offset_succ, F.capacity_eq_pow h3] at h2
      exact beBytes_lt_256 (by omega) b h

end Family

end Bijou
