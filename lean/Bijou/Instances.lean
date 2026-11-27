module

public import Bijou.Canonical
public import Bijou.Order

@[expose] public section

/-!
# The three specified width variants

`bijou32`, `bijou64`, and `bijou128` instantiate the family, and every
test vector from the three SPEC documents is checked by the kernel
(`decide +kernel`), each as a named theorem so `AxiomAudit` covers it. The general theorems (`decode_encode`, `decode_ok`,
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

theorem vec_bijou32_threshold_1 : bijou32.threshold = 252 := by decide +kernel
theorem vec_bijou64_threshold_1 : bijou64.threshold = 248 := by decide +kernel
theorem vec_bijou128_threshold_1 : bijou128.threshold = 240 := by decide +kernel

theorem vec_param_1 : 256 ^ bijou32.tiers = 2 ^ 32 := by decide +kernel
theorem vec_param_2 : 256 ^ bijou64.tiers = 2 ^ 64 := by decide +kernel
theorem vec_param_3 : 256 ^ bijou128.tiers = 2 ^ 128 := by decide +kernel

/-! ## Maximum encoding length (SPEC "maximum encoding length") -/

theorem vec_bijou32_maxBytes_1 : bijou32.maxBytes = 5 := by decide +kernel
theorem vec_bijou64_maxBytes_1 : bijou64.maxBytes = 9 := by decide +kernel
theorem vec_bijou128_maxBytes_1 : bijou128.maxBytes = 17 := by decide +kernel

/-! ## bijou64 offset table (specs/bijou64.md "Offset Table") -/

theorem vec_bijou64_offset_1 : bijou64.offset 1 = 0xF8 := by decide +kernel
theorem vec_bijou64_offset_2 : bijou64.offset 2 = 0x1F8 := by decide +kernel
theorem vec_bijou64_offset_3 : bijou64.offset 3 = 0x101F8 := by decide +kernel
theorem vec_bijou64_offset_4 : bijou64.offset 4 = 0x10101F8 := by decide +kernel
theorem vec_bijou64_offset_5 : bijou64.offset 5 = 0x1010101F8 := by decide +kernel
theorem vec_bijou64_offset_6 : bijou64.offset 6 = 0x101010101F8 := by decide +kernel
theorem vec_bijou64_offset_7 : bijou64.offset 7 = 0x10101010101F8 := by decide +kernel
theorem vec_bijou64_offset_8 : bijou64.offset 8 = 0x1010101010101F8 := by decide +kernel

/-! ## bijou32 / bijou128 offset tables (specs/bijou32.md, specs/bijou128.md vector tables) -/

theorem vec_bijou32_offset_1 : bijou32.offset 1 = 252 := by decide +kernel
theorem vec_bijou32_offset_2 : bijou32.offset 2 = 508 := by decide +kernel
theorem vec_bijou32_offset_3 : bijou32.offset 3 = 66044 := by decide +kernel
theorem vec_bijou32_offset_4 : bijou32.offset 4 = 16843260 := by decide +kernel

theorem vec_bijou128_offset_1 : bijou128.offset 1 = 240 := by decide +kernel
theorem vec_bijou128_offset_2 : bijou128.offset 2 = 496 := by decide +kernel
theorem vec_bijou128_offset_3 : bijou128.offset 3 = 66032 := by decide +kernel
theorem vec_bijou128_offset_4 : bijou128.offset 4 = 16843248 := by decide +kernel

/-! ## bijou64 test vectors (specs/bijou64.md "Test Vectors") -/

theorem vec_bijou64_encode_1 : bijou64.encode 0 = [0x00] := by decide +kernel
theorem vec_bijou64_encode_2 : bijou64.encode 1 = [0x01] := by decide +kernel
theorem vec_bijou64_encode_3 : bijou64.encode 42 = [0x2A] := by decide +kernel
theorem vec_bijou64_encode_4 : bijou64.encode 247 = [0xF7] := by decide +kernel
theorem vec_bijou64_encode_5 : bijou64.encode 248 = [0xF8, 0x00] := by decide +kernel
theorem vec_bijou64_encode_6 : bijou64.encode 300 = [0xF8, 0x34] := by decide +kernel
theorem vec_bijou64_encode_7 : bijou64.encode 503 = [0xF8, 0xFF] := by decide +kernel
theorem vec_bijou64_encode_8 : bijou64.encode 504 = [0xF9, 0x00, 0x00] := by decide +kernel
theorem vec_bijou64_encode_9 : bijou64.encode 1000 = [0xF9, 0x01, 0xF0] := by decide +kernel
theorem vec_bijou64_encode_10 : bijou64.encode 65535 = [0xF9, 0xFE, 0x07] := by decide +kernel
theorem vec_bijou64_encode_11 : bijou64.encode 66039 = [0xF9, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou64_encode_12 : bijou64.encode 66040 = [0xFA, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou64_encode_13 : bijou64.encode 67000 = [0xFA, 0x00, 0x03, 0xC0] := by decide +kernel
theorem vec_bijou64_encode_14 : bijou64.encode 16843255 = [0xFA, 0xFF, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou64_encode_15 : bijou64.encode 16843256 = [0xFB, 0x00, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou64_encode_16 : bijou64.encode 4311810551 = [0xFB, 0xFF, 0xFF, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou64_encode_17 : bijou64.encode 72340172838076920
  = [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou64_encode_18 : bijou64.encode 18446744073709551615
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07] := by decide +kernel

theorem vec_bijou64_decode_1 : bijou64.decode [0xF8, 0x34, 0xFF] = .ok (300, 2) := by decide +kernel
theorem vec_bijou64_decode_2 : bijou64.decode [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]
  = .ok (18446744073709551615, 9) := by decide +kernel

/-! ## bijou64 error vectors (specs/bijou64.md "Error Test Vectors") -/

theorem vec_bijou64_decode_3 : bijou64.decode [] = .error .bufferTooShort := by decide +kernel
theorem vec_bijou64_decode_4 : bijou64.decode [0xF9, 0x00] = .error .bufferTooShort := by decide +kernel
theorem vec_bijou64_decode_5 : bijou64.decode [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  = .error .overflow := by decide +kernel

/-! ## No overlong encodings — executable evidence

In VARU64, `[0xF8, 0x00]` is an overlong encoding of `0` that the
decoder must actively reject; forgetting the check silently accepts
it. In bijou the same bytes simply *mean a different value* — the tier
offset is load-bearing, so there is nothing to reject and nothing to
forget (specs/bijou64.md "Canonicality").
-/

theorem vec_bijou64_decode_6 : bijou64.decode [0xF8, 0x00] = .ok (248, 2) := by decide +kernel
theorem vec_bijou64_decode_7 : bijou64.decode [0xF9, 0x00, 0x00] = .ok (504, 3) := by decide +kernel
theorem vec_bijou64_decode_8 : bijou64.decode [0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
  = .ok (72340172838076920, 9) := by decide +kernel
theorem vec_bijou32_decode_1 : bijou32.decode [0xFC, 0x00] = .ok (252, 2) := by decide +kernel
theorem vec_bijou128_decode_1 : bijou128.decode [0xF0, 0x00] = .ok (240, 2) := by decide +kernel

/-! ## The width variants are not wire-compatible (specs/bijou32.md
"Relationship to bijou64", specs/bijou128.md "Comparison") -/

theorem vec_wire_bijou32_FA : bijou32.decode [0xFA] = .ok (250, 1) := by decide +kernel
theorem vec_wire_bijou64_FA : bijou64.decode [0xFA] = .error .bufferTooShort := by decide +kernel
theorem vec_wire_bijou32_251 : bijou32.encodedLen 251 = 1 := by decide +kernel
theorem vec_wire_bijou64_251 : bijou64.encodedLen 251 = 2 := by decide +kernel
theorem vec_wire_bijou64_247 : bijou64.encodedLen 247 = 1 := by decide +kernel
theorem vec_wire_bijou128_240 : bijou128.encodedLen 240 = 2 := by decide +kernel
theorem vec_wire_bijou128_247 : bijou128.encodedLen 247 = 2 := by decide +kernel

/-! ## bijou32 test vectors (specs/bijou32.md) -/

theorem vec_bijou32_encode_1 : bijou32.encode 0 = [0x00] := by decide +kernel
theorem vec_bijou32_encode_2 : bijou32.encode 1 = [0x01] := by decide +kernel
theorem vec_bijou32_encode_3 : bijou32.encode 42 = [0x2A] := by decide +kernel
theorem vec_bijou32_encode_4 : bijou32.encode 247 = [0xF7] := by decide +kernel
theorem vec_bijou32_encode_5 : bijou32.encode 251 = [0xFB] := by decide +kernel
theorem vec_bijou32_encode_6 : bijou32.encode 252 = [0xFC, 0x00] := by decide +kernel
theorem vec_bijou32_encode_7 : bijou32.encode 300 = [0xFC, 0x30] := by decide +kernel
theorem vec_bijou32_encode_8 : bijou32.encode 507 = [0xFC, 0xFF] := by decide +kernel
theorem vec_bijou32_encode_9 : bijou32.encode 508 = [0xFD, 0x00, 0x00] := by decide +kernel
theorem vec_bijou32_encode_10 : bijou32.encode 1000 = [0xFD, 0x01, 0xEC] := by decide +kernel
theorem vec_bijou32_encode_11 : bijou32.encode 65535 = [0xFD, 0xFE, 0x03] := by decide +kernel
theorem vec_bijou32_encode_12 : bijou32.encode 66043 = [0xFD, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou32_encode_13 : bijou32.encode 66044 = [0xFE, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou32_encode_14 : bijou32.encode 67000 = [0xFE, 0x00, 0x03, 0xBC] := by decide +kernel
theorem vec_bijou32_encode_15 : bijou32.encode 16843259 = [0xFE, 0xFF, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou32_encode_16 : bijou32.encode 16843260 = [0xFF, 0x00, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou32_encode_17 : bijou32.encode 4294967295 = [0xFF, 0xFE, 0xFE, 0xFE, 0x03] := by decide +kernel

theorem vec_bijou32_decode_2 : bijou32.decode [] = .error .bufferTooShort := by decide +kernel
theorem vec_bijou32_decode_3 : bijou32.decode [0xFD, 0x00] = .error .bufferTooShort := by decide +kernel
theorem vec_bijou32_decode_4 : bijou32.decode [0xFF, 0xFF, 0xFF, 0xFF, 0xFF] = .error .overflow := by decide +kernel

/-! ## bijou128 test vectors (specs/bijou128.md) -/

theorem vec_bijou128_encode_1 : bijou128.encode 0 = [0x00] := by decide +kernel
theorem vec_bijou128_encode_2 : bijou128.encode 1 = [0x01] := by decide +kernel
theorem vec_bijou128_encode_3 : bijou128.encode 42 = [0x2A] := by decide +kernel
theorem vec_bijou128_encode_4 : bijou128.encode 239 = [0xEF] := by decide +kernel
theorem vec_bijou128_encode_5 : bijou128.encode 240 = [0xF0, 0x00] := by decide +kernel
theorem vec_bijou128_encode_6 : bijou128.encode 241 = [0xF0, 0x01] := by decide +kernel
theorem vec_bijou128_encode_7 : bijou128.encode 495 = [0xF0, 0xFF] := by decide +kernel
theorem vec_bijou128_encode_8 : bijou128.encode 496 = [0xF1, 0x00, 0x00] := by decide +kernel
theorem vec_bijou128_encode_9 : bijou128.encode 65535 = [0xF1, 0xFE, 0x0F] := by decide +kernel
theorem vec_bijou128_encode_10 : bijou128.encode 66031 = [0xF1, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou128_encode_11 : bijou128.encode 66032 = [0xF2, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou128_encode_12 : bijou128.encode 67000 = [0xF2, 0x00, 0x03, 0xC8] := by decide +kernel
theorem vec_bijou128_encode_13 : bijou128.encode 16843247 = [0xF2, 0xFF, 0xFF, 0xFF] := by decide +kernel
theorem vec_bijou128_encode_14 : bijou128.encode 16843248 = [0xF3, 0x00, 0x00, 0x00, 0x00] := by decide +kernel
theorem vec_bijou128_encode_15 : bijou128.encode (2 ^ 32 - 1) = [0xF3, 0xFE, 0xFE, 0xFE, 0x0F] := by decide +kernel
theorem vec_bijou128_encode_16 : bijou128.encode (2 ^ 64 - 1)
  = [0xF7, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F] := by decide +kernel
theorem vec_bijou128_encode_17 : bijou128.encode (2 ^ 128 - 1)
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F] := by decide +kernel

theorem vec_bijou128_decode_2 : bijou128.decode [] = .error .bufferTooShort := by decide +kernel
theorem vec_bijou128_decode_3 : bijou128.decode [0xF1, 0x00] = .error .bufferTooShort := by decide +kernel
theorem vec_bijou128_decode_4 : bijou128.decode
    [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
     0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  = .error .overflow := by decide +kernel

end Bijou
