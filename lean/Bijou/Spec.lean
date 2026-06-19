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

-- Needed so the decode test vectors in `Bijou.Instances` (whose results
-- are `Except DecodeError (Nat × Nat)`) can be checked with `#guard`.
-- Lean core derives `DecidableEq` for `Except` only on demand.
deriving instance DecidableEq for Except

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

/-- Values below the threshold encode as the single byte equal to the
value (SPEC: "values 0–247 are encoded as a single byte equal to the
value"). -/
theorem encode_lt_threshold {v : Nat} (h : v < F.threshold) : F.encode v = [v] := by
  simp [encode, h]

/-- Every encoding is at least one byte long. -/
theorem encodedLen_pos (v : Nat) : 1 ≤ F.encodedLen v := by
  simp only [encodedLen]
  split <;> omega

/-- Maximum encoding length, as a parameter of the family: `tiers + 1`
bytes (9 for bijou64, 5 for bijou32, 17 for bijou128). -/
def maxBytes : Nat := F.tiers + 1

/-- The encoding never exceeds `maxBytes` (SPEC: "maximum encoding
length is 9 bytes"). -/
theorem encodedLen_le (v : Nat) : F.encodedLen v ≤ F.maxBytes := by
  have := F.tierOf_le v
  simp only [encodedLen, maxBytes]
  split <;> omega

/-- The total encoding length, recovered from the tag byte alone. -/
def lenFromTag (tag : Nat) : Nat :=
  if tag < F.threshold then 1 else tag - F.threshold + 2

/-- Length is determined entirely by the first byte: the count of bytes
a successful `decode` consumes is a function of the tag alone, with no
inspection of the payload. This is the SPEC design goal that enables
`O(1)` skipping and streaming framing. -/
theorem decode_consumed_from_tag {tag : Nat} {rest : List Nat} {v n : Nat}
    (h : F.decode (tag :: rest) = .ok (v, n)) : n = F.lenFromTag tag := by
  simp only [decode] at h
  simp only [lenFromTag]
  by_cases htag : tag < F.threshold
  · rw [if_pos htag] at h ⊢
    simp only [Except.ok.injEq, Prod.mk.injEq] at h
    omega
  · rw [if_neg htag] at h ⊢
    by_cases hlen : tag - F.threshold + 1 ≤ rest.length
    · rw [if_pos hlen] at h
      by_cases hov :
          F.offset (tag - F.threshold + 1) + fromBe (rest.take (tag - F.threshold + 1))
            < 256 ^ F.tiers
      · rw [if_pos hov] at h
        simp only [Except.ok.injEq, Prod.mk.injEq] at h
        omega
      · rw [if_neg hov] at h; simp at h
    · rw [if_neg hlen] at h; simp at h

end Family

end Bijou
