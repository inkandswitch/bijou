import Bijou.Canonical
import Bijou.Order
import Bijou.RoundTrip

/-!
# The signed formats: zigzag ∘ unsigned

The signed bijou formats (`bijou32s`, `bijou64s`, `bijou128s`) are
defined as the composition of the standard zigzag bijection with the
corresponding unsigned format — no additional framing, flags, or bias
constants (SPEC: specs/bijou{32,64,128}s.md).

This module proves the zigzag layer is a bijection between `Int` and
`Nat` (`unzigzag_zigzag` / `zigzag_unzigzag`), characterizes its
interaction with the value domain (`zigzag_lt_two_mul_iff`), and lifts
every headline unsigned theorem through the composition:

- `decodeS_encodeS` — signed round-trip, with or without trailing bytes.
- `decodeS_ok` — canonicality: any accepted byte string is exactly
  `encodeS` of the returned value, which lies in the signed domain.
- `encodeS_injective` — distinct signed values never share an encoding.
- `encodedLenS_le` — signed encodings never exceed `maxBytes`.
- `encodeS_lex_iff` — byte order is **zigzag order** (magnitude, with
  the negative preceding the positive at equal magnitude), *not*
  numeric order — exactly as the SPEC's Ordering section warns.
-/

namespace Bijou

/-- The zigzag map: `0, -1, 1, -2, 2, …` ↦ `0, 1, 2, 3, 4, …`.

Arithmetic characterization of the SPEC's
`zigzag(n) = (n << 1) XOR (n >> 63)`: nonnegative `n` maps to `2 * n`,
negative `n` maps to `2 * |n| - 1`. -/
def zigzag : Int → Nat
  | .ofNat n => 2 * n
  | .negSucc n => 2 * n + 1

/-- The inverse zigzag map (`unzigzag(z) = (z >> 1) XOR -(z AND 1)`):
even `z` maps to `z / 2`, odd `z` to `-(z / 2 + 1)`. -/
def unzigzag (z : Nat) : Int :=
  if z % 2 = 0 then .ofNat (z / 2) else .negSucc (z / 2)

@[simp]
theorem unzigzag_zigzag (i : Int) : unzigzag (zigzag i) = i := by
  cases i with
  | ofNat n =>
    simp only [zigzag, unzigzag]
    rw [if_pos (by omega)]
    congr 1
    omega
  | negSucc n =>
    simp only [zigzag, unzigzag]
    rw [if_neg (by omega)]
    congr 1
    omega

@[simp]
theorem zigzag_unzigzag (z : Nat) : zigzag (unzigzag z) = z := by
  simp only [unzigzag]
  split
  · simp only [zigzag]
    omega
  · simp only [zigzag]
    omega

theorem zigzag_injective {i j : Int} (h : zigzag i = zigzag j) : i = j := by
  have := congrArg unzigzag h
  rwa [unzigzag_zigzag, unzigzag_zigzag] at this

/-- Zigzag maps the symmetric interval `[-h, h)` exactly onto
`[0, 2 * h)`: the signed domain of a width tiles the unsigned domain
with nothing left over — this is what makes the composition total *and*
bijective. -/
theorem zigzag_lt_two_mul_iff (i : Int) (h : Nat) :
    zigzag i < 2 * h ↔ -(h : Int) ≤ i ∧ i < (h : Int) := by
  cases i with
  | ofNat n =>
    simp only [zigzag, Int.ofNat_eq_natCast]
    omega
  | negSucc n =>
    simp only [zigzag, Int.negSucc_eq]
    omega

/-- Zigzag order: magnitude first, the negative before the positive at
equal magnitude. This is the order that byte-lexicographic comparison
of signed encodings realizes (`encodeS_lex_iff`). -/
theorem zigzag_lt_zigzag_iff (i j : Int) :
    zigzag i < zigzag j ↔
      i.natAbs < j.natAbs ∨ (i.natAbs = j.natAbs ∧ i < j) := by
  cases i <;> cases j <;>
    simp only [zigzag, Int.natAbs, Int.negSucc_eq, Int.ofNat_eq_natCast] <;>
    omega

namespace Family

variable (F : Family)

/-- Half the unsigned value domain: the count of nonnegative signed
values, and the magnitude bound of the signed domain `[-half, half)`.
`2^31` / `2^63` / `2^127` for the three widths. -/
def half : Nat := 256 ^ F.tiers / 2

theorem two_mul_half : 2 * F.half = 256 ^ F.tiers := by
  obtain ⟨t, ht⟩ : ∃ t, F.tiers = t + 1 :=
    ⟨F.tiers - 1, by have := F.tiers_pos; omega⟩
  have hx : 0 < 256 ^ t := Nat.pow_pos (by omega)
  simp only [half, ht, Nat.pow_succ]
  omega

/-- Encode a signed value: zigzag, then the unsigned format
(SPEC: `encode_s(n) = encode_u(zigzag(n))`). -/
def encodeS (i : Int) : List Nat := F.encode (zigzag i)

/-- The number of bytes `encodeS i` produces. -/
def encodedLenS (i : Int) : Nat := F.encodedLen (zigzag i)

/-- Decode a signed value from the front of `bs`
(SPEC: `decode_s(b) = unzigzag(decode_u(b))`). The error conditions are
exactly the unsigned format's — there are no signed-specific errors. -/
def decodeS (bs : List Nat) : Except DecodeError (Int × Nat) :=
  match F.decode bs with
  | .ok (z, n) => .ok (unzigzag z, n)
  | .error e => .error e

/-- Values in the signed domain zigzag into the unsigned domain. -/
theorem zigzag_lt_pow {i : Int}
    (h₁ : -(F.half : Int) ≤ i) (h₂ : i < (F.half : Int)) :
    zigzag i < 256 ^ F.tiers := by
  rw [← F.two_mul_half]
  exact (zigzag_lt_two_mul_iff i F.half).mpr ⟨h₁, h₂⟩

@[simp]
theorem encodeS_length (i : Int) : (F.encodeS i).length = F.encodedLenS i :=
  F.encode_length (zigzag i)

/-- Signed encodings never exceed `maxBytes`. -/
theorem encodedLenS_le (i : Int) : F.encodedLenS i ≤ F.maxBytes :=
  F.encodedLen_le (zigzag i)

/-- Signed round-trip: decoding an encoding returns the original value
and consumes exactly its bytes, with or without trailing data. -/
theorem decodeS_encodeS {i : Int}
    (h₁ : -(F.half : Int) ≤ i) (h₂ : i < (F.half : Int)) (rest : List Nat) :
    F.decodeS (F.encodeS i ++ rest) = .ok (i, F.encodedLenS i) := by
  simp only [decodeS, encodeS, encodedLenS]
  rw [F.decode_encode (F.zigzag_lt_pow h₁ h₂) rest]
  simp

theorem decodeS_encodeS' {i : Int}
    (h₁ : -(F.half : Int) ≤ i) (h₂ : i < (F.half : Int)) :
    F.decodeS (F.encodeS i) = .ok (i, F.encodedLenS i) := by
  have := F.decodeS_encodeS h₁ h₂ []
  rwa [List.append_nil] at this

/-- Signed canonicality: if `decodeS bs = ok (i, n)` then `i` lies in
the signed domain and the `n` bytes consumed are exactly `encodeS i`.
The decoder only ever accepts canonical encodings. -/
theorem decodeS_ok {bs : List Nat} {i : Int} {n : Nat}
    (hbytes : ∀ b ∈ bs, b < 256)
    (h : F.decodeS bs = .ok (i, n)) :
    (-(F.half : Int) ≤ i ∧ i < (F.half : Int))
      ∧ n = F.encodedLenS i ∧ bs.take n = F.encodeS i := by
  cases hd : F.decode bs with
  | error e => simp [decodeS, hd] at h
  | ok p =>
    obtain ⟨z, m⟩ := p
    simp only [decodeS, hd, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨hi, hm⟩ := h
    subst hi
    subst hm
    obtain ⟨hz, hn, htake⟩ := F.decode_ok hbytes hd
    have hbound : zigzag (unzigzag z) < 2 * F.half := by
      rw [zigzag_unzigzag, F.two_mul_half]
      exact hz
    refine ⟨(zigzag_lt_two_mul_iff (unzigzag z) F.half).mp hbound, ?_, ?_⟩
    · simpa [encodedLenS] using hn
    · simpa [encodeS] using htake

/-- Distinct signed values never share an encoding. -/
theorem encodeS_injective {i j : Int}
    (hi₁ : -(F.half : Int) ≤ i) (hi₂ : i < (F.half : Int))
    (hj₁ : -(F.half : Int) ≤ j) (hj₂ : j < (F.half : Int))
    (h : F.encodeS i = F.encodeS j) : i = j :=
  zigzag_injective
    (F.encode_injective (F.zigzag_lt_pow hi₁ hi₂) (F.zigzag_lt_pow hj₁ hj₂) h)

/-- Byte-lexicographic order of signed encodings is **zigzag order**
(magnitude, negative first at equal magnitude), *not* numeric order.
Consumers requiring memcomparable signed keys must not use the signed
formats (SPEC "Ordering"). -/
theorem encodeS_lex_iff {i j : Int}
    (hi₁ : -(F.half : Int) ≤ i) (hi₂ : i < (F.half : Int))
    (hj₁ : -(F.half : Int) ≤ j) (hj₂ : j < (F.half : Int)) :
    Lex (F.encodeS i) (F.encodeS j) ↔
      i.natAbs < j.natAbs ∨ (i.natAbs = j.natAbs ∧ i < j) := by
  rw [← zigzag_lt_zigzag_iff]
  exact
    (F.encode_lex_iff (F.zigzag_lt_pow hi₁ hi₂) (F.zigzag_lt_pow hj₁ hj₂)).symm

end Family

end Bijou
