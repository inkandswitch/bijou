import Bijou.Canonical
import Bijou.Order

/-!
# The three specified width variants

`bijou32`, `bijou64`, and `bijou128` instantiate the family, and every
test vector from the three SPEC documents is checked by reduction
(`rfl`). The general theorems (`decode_encode`, `decode_ok`,
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

example : bijou32.threshold = 252 := rfl
example : bijou64.threshold = 248 := rfl
example : bijou128.threshold = 240 := rfl

example : 256 ^ bijou32.tiers = 2 ^ 32 := by decide
example : 256 ^ bijou64.tiers = 2 ^ 64 := by decide
example : 256 ^ bijou128.tiers = 2 ^ 128 := by decide

/-! ## bijou64 offset table (SPEC.md "Offset Table") -/

example : bijou64.offset 1 = 0xF8 := by decide
example : bijou64.offset 2 = 0x1F8 := by decide
example : bijou64.offset 3 = 0x101F8 := by decide
example : bijou64.offset 4 = 0x10101F8 := by decide
example : bijou64.offset 5 = 0x1010101F8 := by decide
example : bijou64.offset 6 = 0x101010101F8 := by decide
example : bijou64.offset 7 = 0x10101010101F8 := by decide
example : bijou64.offset 8 = 0x1010101010101F8 := by decide

/-! ## bijou64 test vectors (SPEC.md "Test Vectors") -/

example : bijou64.encode 0 = [0x00] := rfl
example : bijou64.encode 1 = [0x01] := rfl
example : bijou64.encode 42 = [0x2A] := rfl
example : bijou64.encode 247 = [0xF7] := rfl
example : bijou64.encode 248 = [0xF8, 0x00] := rfl
example : bijou64.encode 300 = [0xF8, 0x34] := rfl
example : bijou64.encode 503 = [0xF8, 0xFF] := rfl
example : bijou64.encode 504 = [0xF9, 0x00, 0x00] := rfl
example : bijou64.encode 1000 = [0xF9, 0x01, 0xF0] := rfl
example : bijou64.encode 65535 = [0xF9, 0xFE, 0x07] := rfl
example : bijou64.encode 66039 = [0xF9, 0xFF, 0xFF] := rfl
example : bijou64.encode 66040 = [0xFA, 0x00, 0x00, 0x00] := rfl
example : bijou64.encode 67000 = [0xFA, 0x00, 0x03, 0xC0] := rfl
example : bijou64.encode 16843255 = [0xFA, 0xFF, 0xFF, 0xFF] := rfl
example : bijou64.encode 16843256 = [0xFB, 0x00, 0x00, 0x00, 0x00] := rfl
example : bijou64.encode 4311810551 = [0xFB, 0xFF, 0xFF, 0xFF, 0xFF] := rfl
example : bijou64.encode 72340172838076920
    = [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] := rfl
example : bijou64.encode 18446744073709551615
    = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07] := rfl

example : bijou64.decode [0xF8, 0x34, 0xFF] = .ok (300, 2) := rfl
example : bijou64.decode [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]
    = .ok (18446744073709551615, 9) := rfl

/-! ## bijou64 error vectors (SPEC.md "Error Test Vectors") -/

example : bijou64.decode [] = .error .bufferTooShort := rfl
example : bijou64.decode [0xF9, 0x00] = .error .bufferTooShort := rfl
example : bijou64.decode [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    = .error .overflow := rfl

/-! ## bijou32 test vectors (bijou32/SPEC.md) -/

example : bijou32.encode 0 = [0x00] := rfl
example : bijou32.encode 1 = [0x01] := rfl
example : bijou32.encode 42 = [0x2A] := rfl
example : bijou32.encode 247 = [0xF7] := rfl
example : bijou32.encode 251 = [0xFB] := rfl
example : bijou32.encode 252 = [0xFC, 0x00] := rfl
example : bijou32.encode 300 = [0xFC, 0x30] := rfl
example : bijou32.encode 507 = [0xFC, 0xFF] := rfl
example : bijou32.encode 508 = [0xFD, 0x00, 0x00] := rfl
example : bijou32.encode 1000 = [0xFD, 0x01, 0xEC] := rfl
example : bijou32.encode 65535 = [0xFD, 0xFE, 0x03] := rfl
example : bijou32.encode 66043 = [0xFD, 0xFF, 0xFF] := rfl
example : bijou32.encode 66044 = [0xFE, 0x00, 0x00, 0x00] := rfl
example : bijou32.encode 67000 = [0xFE, 0x00, 0x03, 0xBC] := rfl
example : bijou32.encode 16843259 = [0xFE, 0xFF, 0xFF, 0xFF] := rfl
example : bijou32.encode 16843260 = [0xFF, 0x00, 0x00, 0x00, 0x00] := rfl
example : bijou32.encode 4294967295 = [0xFF, 0xFE, 0xFE, 0xFE, 0x03] := rfl

example : bijou32.decode [] = .error .bufferTooShort := rfl
example : bijou32.decode [0xFD, 0x00] = .error .bufferTooShort := rfl
example : bijou32.decode [0xFF, 0xFF, 0xFF, 0xFF, 0xFF] = .error .overflow := rfl

/-! ## bijou128 test vectors (bijou128/SPEC.md) -/

example : bijou128.encode 0 = [0x00] := rfl
example : bijou128.encode 1 = [0x01] := rfl
example : bijou128.encode 42 = [0x2A] := rfl
example : bijou128.encode 239 = [0xEF] := rfl
example : bijou128.encode 240 = [0xF0, 0x00] := rfl
example : bijou128.encode 241 = [0xF0, 0x01] := rfl
example : bijou128.encode 495 = [0xF0, 0xFF] := rfl
example : bijou128.encode 496 = [0xF1, 0x00, 0x00] := rfl
example : bijou128.encode 65535 = [0xF1, 0xFE, 0x0F] := rfl
example : bijou128.encode 66031 = [0xF1, 0xFF, 0xFF] := rfl
example : bijou128.encode 66032 = [0xF2, 0x00, 0x00, 0x00] := rfl
example : bijou128.encode 67000 = [0xF2, 0x00, 0x03, 0xC8] := rfl
example : bijou128.encode 16843247 = [0xF2, 0xFF, 0xFF, 0xFF] := rfl
example : bijou128.encode 16843248 = [0xF3, 0x00, 0x00, 0x00, 0x00] := rfl
example : bijou128.encode (2 ^ 32 - 1) = [0xF3, 0xFE, 0xFE, 0xFE, 0x0F] := rfl
example : bijou128.encode (2 ^ 64 - 1)
    = [0xF7, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F] := rfl
example : bijou128.encode (2 ^ 128 - 1)
    = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
       0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F] := rfl

example : bijou128.decode [] = .error .bufferTooShort := rfl
example : bijou128.decode [0xF1, 0x00] = .error .bufferTooShort := rfl
example : bijou128.decode
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
     0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    = .error .overflow := rfl

end Bijou
