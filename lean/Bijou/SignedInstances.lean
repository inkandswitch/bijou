import Bijou.Instances
import Bijou.Signed

/-!
# The three signed width variants

`bijou32s`, `bijou64s`, and `bijou128s` are the signed views of the
three unsigned instances (`Family.encodeS` / `Family.decodeS` on
`bijou32` / `bijou64` / `bijou128`). Every test vector from the three
signed SPEC documents (specs/bijou{32,64,128}s.md) is checked at
compile time with `#guard`, including the error vectors. The general
signed theorems (`decodeS_encodeS`, `decodeS_ok`, `encodeS_injective`,
`encodeS_lex_iff`, `encodedLenS_le`) apply to all three instances
directly.
-/

namespace Bijou

/-! ## Signed domains match the SPECs -/

#guard bijou32.half = 2 ^ 31
#guard bijou64.half = 2 ^ 63
#guard bijou128.half = 2 ^ 127

/-! ## The zigzag table (bijou64s SPEC "The Zigzag Map") -/

#guard zigzag 0 = 0
#guard zigzag (-1) = 1
#guard zigzag 1 = 2
#guard zigzag (-2) = 3
#guard zigzag 2 = 4
#guard zigzag (-124) = 247
#guard zigzag (2 ^ 63 - 1) = 2 ^ 64 - 2
#guard zigzag (-(2 ^ 63)) = 2 ^ 64 - 1

/-! ## bijou64s test vectors (specs/bijou64s.md) -/

#guard bijou64.encodeS 0 = [0x00]
#guard bijou64.encodeS (-1) = [0x01]
#guard bijou64.encodeS 1 = [0x02]
#guard bijou64.encodeS (-2) = [0x03]
#guard bijou64.encodeS 2 = [0x04]
#guard bijou64.encodeS 123 = [0xF6]
#guard bijou64.encodeS (-124) = [0xF7]
#guard bijou64.encodeS 124 = [0xF8, 0x00]
#guard bijou64.encodeS (-125) = [0xF8, 0x01]
#guard bijou64.encodeS 251 = [0xF8, 0xFE]
#guard bijou64.encodeS (-252) = [0xF8, 0xFF]
#guard bijou64.encodeS 252 = [0xF9, 0x00, 0x00]
#guard bijou64.encodeS (2 ^ 63 - 1)
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x06]
#guard bijou64.encodeS (-(2 ^ 63))
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]

/-! ### Signed decode round-trips the vectors -/

#guard bijou64.decodeS [0x00] = .ok (0, 1)
#guard bijou64.decodeS [0x01] = .ok (-1, 1)
#guard bijou64.decodeS [0xF8, 0x01] = .ok (-125, 2)
#guard bijou64.decodeS [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]
  = .ok (-(2 ^ 63), 9)

/-! ### bijou64s error vectors -/

#guard bijou64.decodeS [] = .error .bufferTooShort
#guard bijou64.decodeS [0xF8] = .error .bufferTooShort
#guard bijou64.decodeS [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  = .error .overflow

/-! ## bijou32s test vectors (specs/bijou32s.md) -/

#guard bijou32.encodeS 0 = [0x00]
#guard bijou32.encodeS (-1) = [0x01]
#guard bijou32.encodeS 1 = [0x02]
#guard bijou32.encodeS 125 = [0xFA]
#guard bijou32.encodeS (-126) = [0xFB]
#guard bijou32.encodeS 126 = [0xFC, 0x00]
#guard bijou32.encodeS (-127) = [0xFC, 0x01]
#guard bijou32.encodeS 253 = [0xFC, 0xFE]
#guard bijou32.encodeS (-254) = [0xFC, 0xFF]
#guard bijou32.encodeS 254 = [0xFD, 0x00, 0x00]
#guard bijou32.encodeS (2 ^ 31 - 1) = [0xFF, 0xFE, 0xFE, 0xFE, 0x02]
#guard bijou32.encodeS (-(2 ^ 31)) = [0xFF, 0xFE, 0xFE, 0xFE, 0x03]

/-! ## bijou128s test vectors (specs/bijou128s.md) -/

#guard bijou128.encodeS 0 = [0x00]
#guard bijou128.encodeS (-1) = [0x01]
#guard bijou128.encodeS 1 = [0x02]
#guard bijou128.encodeS 119 = [0xEE]
#guard bijou128.encodeS (-120) = [0xEF]
#guard bijou128.encodeS 120 = [0xF0, 0x00]
#guard bijou128.encodeS (-121) = [0xF0, 0x01]
#guard bijou128.encodeS 247 = [0xF0, 0xFE]
#guard bijou128.encodeS (-248) = [0xF0, 0xFF]
#guard bijou128.encodeS 248 = [0xF1, 0x00, 0x00]
#guard bijou128.encodeS (2 ^ 127 - 1)
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0E]
#guard bijou128.encodeS (-(2 ^ 127))
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F]

/-! ## The signed tier table (bijou64s SPEC "Signed Tier Table")

Boundary values of each row: encoded length changes exactly at the
documented magnitudes. -/

#guard bijou64.encodedLenS (-124) = 1
#guard bijou64.encodedLenS 123 = 1
#guard bijou64.encodedLenS (-125) = 2
#guard bijou64.encodedLenS 124 = 2
#guard bijou64.encodedLenS (-252) = 2
#guard bijou64.encodedLenS 251 = 2
#guard bijou64.encodedLenS (-253) = 3
#guard bijou64.encodedLenS 252 = 3
#guard bijou64.encodedLenS (-33020) = 3
#guard bijou64.encodedLenS 33019 = 3
#guard bijou64.encodedLenS (-33021) = 4
#guard bijou64.encodedLenS 33020 = 4
#guard bijou64.encodedLenS (-8421628) = 4
#guard bijou64.encodedLenS 8421627 = 4
#guard bijou64.encodedLenS (-8421629) = 5
#guard bijou64.encodedLenS 8421628 = 5
#guard bijou64.encodedLenS (-2155905276) = 5
#guard bijou64.encodedLenS 2155905275 = 5
#guard bijou64.encodedLenS (-(2 ^ 63)) = 9
#guard bijou64.encodedLenS (2 ^ 63 - 1) = 9

end Bijou
