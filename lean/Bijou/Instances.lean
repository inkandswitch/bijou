import Bijou.Canonical
import Bijou.Order

/-!
# The three specified width variants

`bijou32`, `bijou64`, and `bijou128` instantiate the family, and every
test vector from the three SPEC documents is checked at compile time
with `#guard`. The general theorems (`decode_encode`, `decode_ok`,
`encode_injective`, `decode_canonical`, `encode_lex_iff`) apply to all
three instances directly.
-/

namespace Bijou

/-- bijou32: 4 multi-byte tiers, threshold 252, domain `[0, 2^32)`. -/
def bijou32 : Family := ⟨4, by decide, by decide⟩

/-- bijou64: 8 multi-byte tiers, threshold 248, domain `[0, 2^64)`. -/
def bijou64 : Family := ⟨8, by decide, by decide⟩

/-- bijou128: 16 multi-byte tiers, threshold 240, domain `[0, 2^128)`. -/
def bijou128 : Family := ⟨16, by decide, by decide⟩

/-! ## Derived parameters match the SPECs -/

#guard bijou32.threshold = 252
#guard bijou64.threshold = 248
#guard bijou128.threshold = 240

#guard 256 ^ bijou32.tiers = 2 ^ 32
#guard 256 ^ bijou64.tiers = 2 ^ 64
#guard 256 ^ bijou128.tiers = 2 ^ 128

/-! ## Maximum encoding length (SPEC "maximum encoding length") -/

#guard bijou32.maxBytes = 5
#guard bijou64.maxBytes = 9
#guard bijou128.maxBytes = 17

/-! ## bijou64 offset table (SPEC.md "Offset Table") -/

#guard bijou64.offset 1 = 0xF8
#guard bijou64.offset 2 = 0x1F8
#guard bijou64.offset 3 = 0x101F8
#guard bijou64.offset 4 = 0x10101F8
#guard bijou64.offset 5 = 0x1010101F8
#guard bijou64.offset 6 = 0x101010101F8
#guard bijou64.offset 7 = 0x10101010101F8
#guard bijou64.offset 8 = 0x1010101010101F8

/-! ## bijou32 / bijou128 offset tables (their SPEC.md vector tables) -/

#guard bijou32.offset 1 = 252
#guard bijou32.offset 2 = 508
#guard bijou32.offset 3 = 66044
#guard bijou32.offset 4 = 16843260

#guard bijou128.offset 1 = 240
#guard bijou128.offset 2 = 496
#guard bijou128.offset 3 = 66032
#guard bijou128.offset 4 = 16843248

/-! ## bijou64 test vectors (SPEC.md "Test Vectors") -/

#guard bijou64.encode 0 = [0x00]
#guard bijou64.encode 1 = [0x01]
#guard bijou64.encode 42 = [0x2A]
#guard bijou64.encode 247 = [0xF7]
#guard bijou64.encode 248 = [0xF8, 0x00]
#guard bijou64.encode 300 = [0xF8, 0x34]
#guard bijou64.encode 503 = [0xF8, 0xFF]
#guard bijou64.encode 504 = [0xF9, 0x00, 0x00]
#guard bijou64.encode 1000 = [0xF9, 0x01, 0xF0]
#guard bijou64.encode 65535 = [0xF9, 0xFE, 0x07]
#guard bijou64.encode 66039 = [0xF9, 0xFF, 0xFF]
#guard bijou64.encode 66040 = [0xFA, 0x00, 0x00, 0x00]
#guard bijou64.encode 67000 = [0xFA, 0x00, 0x03, 0xC0]
#guard bijou64.encode 16843255 = [0xFA, 0xFF, 0xFF, 0xFF]
#guard bijou64.encode 16843256 = [0xFB, 0x00, 0x00, 0x00, 0x00]
#guard bijou64.encode 4311810551 = [0xFB, 0xFF, 0xFF, 0xFF, 0xFF]
#guard bijou64.encode 72340172838076920
  = [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
#guard bijou64.encode 18446744073709551615
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]

#guard bijou64.decode [0xF8, 0x34, 0xFF] = .ok (300, 2)
#guard bijou64.decode [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]
  = .ok (18446744073709551615, 9)

/-! ## bijou64 error vectors (SPEC.md "Error Test Vectors") -/

#guard bijou64.decode [] = .error .bufferTooShort
#guard bijou64.decode [0xF9, 0x00] = .error .bufferTooShort
#guard bijou64.decode [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  = .error .overflow

/-! ## No overlong encodings — executable evidence

In VARU64, `[0xF8, 0x00]` is an overlong encoding of `0` that the
decoder must actively reject; forgetting the check silently accepts
it. In bijou the same bytes simply *mean a different value* — the tier
offset is load-bearing, so there is nothing to reject and nothing to
forget (SPEC.md "Canonicality").
-/

#guard bijou64.decode [0xF8, 0x00] = .ok (248, 2)
#guard bijou64.decode [0xF9, 0x00, 0x00] = .ok (504, 3)
#guard bijou64.decode [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
  = .ok (72340172838076920, 9)
#guard bijou32.decode [0xFC, 0x00] = .ok (252, 2)
#guard bijou128.decode [0xF0, 0x00] = .ok (240, 2)

/-! ## bijou32 test vectors (bijou32/SPEC.md) -/

#guard bijou32.encode 0 = [0x00]
#guard bijou32.encode 1 = [0x01]
#guard bijou32.encode 42 = [0x2A]
#guard bijou32.encode 247 = [0xF7]
#guard bijou32.encode 251 = [0xFB]
#guard bijou32.encode 252 = [0xFC, 0x00]
#guard bijou32.encode 300 = [0xFC, 0x30]
#guard bijou32.encode 507 = [0xFC, 0xFF]
#guard bijou32.encode 508 = [0xFD, 0x00, 0x00]
#guard bijou32.encode 1000 = [0xFD, 0x01, 0xEC]
#guard bijou32.encode 65535 = [0xFD, 0xFE, 0x03]
#guard bijou32.encode 66043 = [0xFD, 0xFF, 0xFF]
#guard bijou32.encode 66044 = [0xFE, 0x00, 0x00, 0x00]
#guard bijou32.encode 67000 = [0xFE, 0x00, 0x03, 0xBC]
#guard bijou32.encode 16843259 = [0xFE, 0xFF, 0xFF, 0xFF]
#guard bijou32.encode 16843260 = [0xFF, 0x00, 0x00, 0x00, 0x00]
#guard bijou32.encode 4294967295 = [0xFF, 0xFE, 0xFE, 0xFE, 0x03]

#guard bijou32.decode [] = .error .bufferTooShort
#guard bijou32.decode [0xFD, 0x00] = .error .bufferTooShort
#guard bijou32.decode [0xFF, 0xFF, 0xFF, 0xFF, 0xFF] = .error .overflow

/-! ## bijou128 test vectors (bijou128/SPEC.md) -/

#guard bijou128.encode 0 = [0x00]
#guard bijou128.encode 1 = [0x01]
#guard bijou128.encode 42 = [0x2A]
#guard bijou128.encode 239 = [0xEF]
#guard bijou128.encode 240 = [0xF0, 0x00]
#guard bijou128.encode 241 = [0xF0, 0x01]
#guard bijou128.encode 495 = [0xF0, 0xFF]
#guard bijou128.encode 496 = [0xF1, 0x00, 0x00]
#guard bijou128.encode 65535 = [0xF1, 0xFE, 0x0F]
#guard bijou128.encode 66031 = [0xF1, 0xFF, 0xFF]
#guard bijou128.encode 66032 = [0xF2, 0x00, 0x00, 0x00]
#guard bijou128.encode 67000 = [0xF2, 0x00, 0x03, 0xC8]
#guard bijou128.encode 16843247 = [0xF2, 0xFF, 0xFF, 0xFF]
#guard bijou128.encode 16843248 = [0xF3, 0x00, 0x00, 0x00, 0x00]
#guard bijou128.encode (2 ^ 32 - 1) = [0xF3, 0xFE, 0xFE, 0xFE, 0x0F]
#guard bijou128.encode (2 ^ 64 - 1)
  = [0xF7, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F]
#guard bijou128.encode (2 ^ 128 - 1)
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F]

#guard bijou128.decode [] = .error .bufferTooShort
#guard bijou128.decode [0xF1, 0x00] = .error .bufferTooShort
#guard bijou128.decode
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
     0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  = .error .overflow

end Bijou
