module

public import Bijou.Bytes
public import Bijou.Canonical
public import Bijou.Family
public import Bijou.Instances
public import Bijou.Length
public import Bijou.Order
public import Bijou.RoundTrip
public import Bijou.Signed
public import Bijou.SignedInstances
public import Bijou.Spec

@[expose] public section

/-!
# Bijou: machine-checked format proofs

A Lean 4 model of the bijou family of bijective variable-length
integer encodings, parametrized over the tier count so that one
development covers `bijou32`, `bijou64`, and `bijou128`.

Headline theorems (all in `Bijou.Family`):

- `decode_encode` — round-trip: decoding an encoding returns the
  original value, with or without trailing bytes.
- `decode_ok` — canonicality by construction: any byte string the
  decoder accepts is exactly `encode` of the returned value. There is
  no overlong encoding to reject.
- `encode_injective` / `decode_canonical` / `encode_bijection` —
  `encode` is a bijection between `[0, 256 ^ tiers)` and the accepted
  byte strings.
- `encode_lex_iff` / `encode_lex_trichotomy` — lexicographic byte order
  equals numeric order, and is a strict total order on encodings.
- `decode_consumed_from_tag` — encoding length is a function of the
  first byte alone (O(1) framing).
- `encodedLen_le` — encodings never exceed `maxBytes` (`tiers + 1`).
- `decode_overflow_max_tag` — overflow is possible only at the top tier.
- `encodedLen_mono` / `encodedLen_eq_one_iff` / `encodedLen_eq_succ_iff`
  (`Bijou.Length`) — the tier table in general: which values get how
  many bytes, and longer encodings only for larger values.

Signed formats (`Bijou.Signed`, zigzag ∘ unsigned per the bijou‹N›s
SPECs): `unzigzag_zigzag` / `zigzag_unzigzag` (the zigzag layer is a
bijection), and the lifted family theorems `decodeS_encodeS`,
`decodeS_ok`, `encodeS_injective`, `encodedLenS_le`, and
`encodeS_lex_iff` (byte order is zigzag order — magnitude, negative
first at equal magnitude — not numeric order), plus `encodedLenS_mono` /
`encodedLenS_eq_one_iff` / `encodedLenS_eq_succ_iff` (length grows with
magnitude regardless of sign; the signed tier table in general).

`Bijou.Instances` and `Bijou.SignedInstances` instantiate the six
specified variants and check every test vector from the SPEC documents
(offset tables, `maxBytes`, signed tier boundaries, error vectors) by
kernel reduction (`decide +kernel`), so they are checked like theorems:
no trust in compiled code.
-/
