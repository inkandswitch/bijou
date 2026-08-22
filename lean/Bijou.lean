import Bijou.Bytes
import Bijou.Canonical
import Bijou.Family
import Bijou.Instances
import Bijou.Order
import Bijou.RoundTrip
import Bijou.Spec

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

`Bijou.Instances` instantiates the three specified variants and checks
every test vector from the SPEC documents (and the offset tables and
`maxBytes`) by reduction.
-/
