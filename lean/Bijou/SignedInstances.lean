module

public import Bijou.Instances
public import Bijou.Signed

@[expose] public section

/-!
# The three signed width variants

`bijou32s`, `bijou64s`, and `bijou128s` are the signed views of the
three unsigned instances (`Family.encodeS` / `Family.decodeS` on
`bijou32` / `bijou64` / `bijou128`). Every test vector from the three
signed SPEC documents (specs/bijou{32,64,128}s.md) is checked at
compile time by the kernel (`decide +kernel`), including the error
vectors. The general signed theorems (`decodeS_encodeS`, `decodeS_ok`,
`encodeS_injective`, `encodeS_lex_iff`, `encodedLenS_le`) apply to all
three instances directly.
-/

namespace Bijou

/-! ## Signed domains match the SPECs -/

theorem svec_bijou32_half_1 : bijou32.half = 2 ^ 31 := by decide +kernel
theorem svec_bijou64_half_1 : bijou64.half = 2 ^ 63 := by decide +kernel
theorem svec_bijou128_half_1 : bijou128.half = 2 ^ 127 := by decide +kernel

/-! ## The zigzag table (bijou64s SPEC "The Zigzag Map") -/

theorem svec_zigzag_1 : zigzag 0 = 0 := by decide +kernel
theorem svec_zigzag_2 : zigzag (-1) = 1 := by decide +kernel
theorem svec_zigzag_3 : zigzag 1 = 2 := by decide +kernel
theorem svec_zigzag_4 : zigzag (-2) = 3 := by decide +kernel
theorem svec_zigzag_5 : zigzag 2 = 4 := by decide +kernel
theorem svec_zigzag_6 : zigzag (-124) = 247 := by decide +kernel
theorem svec_zigzag_7 : zigzag (2 ^ 63 - 1) = 2 ^ 64 - 2 := by decide +kernel
theorem svec_zigzag_8 : zigzag (-(2 ^ 63)) = 2 ^ 64 - 1 := by decide +kernel

/-! ## bijou64s test vectors (specs/bijou64s.md) -/

theorem svec_bijou64_encodeS_1 : bijou64.encodeS 0 = [0x00] := by decide +kernel
theorem svec_bijou64_encodeS_2 : bijou64.encodeS (-1) = [0x01] := by decide +kernel
theorem svec_bijou64_encodeS_3 : bijou64.encodeS 1 = [0x02] := by decide +kernel
theorem svec_bijou64_encodeS_4 : bijou64.encodeS (-2) = [0x03] := by decide +kernel
theorem svec_bijou64_encodeS_5 : bijou64.encodeS 2 = [0x04] := by decide +kernel
theorem svec_bijou64_encodeS_6 : bijou64.encodeS 123 = [0xF6] := by decide +kernel
theorem svec_bijou64_encodeS_7 : bijou64.encodeS (-124) = [0xF7] := by decide +kernel
theorem svec_bijou64_encodeS_8 : bijou64.encodeS 124 = [0xF8, 0x00] := by decide +kernel
theorem svec_bijou64_encodeS_9 : bijou64.encodeS (-125) = [0xF8, 0x01] := by decide +kernel
theorem svec_bijou64_encodeS_10 : bijou64.encodeS 251 = [0xF8, 0xFE] := by decide +kernel
theorem svec_bijou64_encodeS_11 : bijou64.encodeS (-252) = [0xF8, 0xFF] := by decide +kernel
theorem svec_bijou64_encodeS_12 : bijou64.encodeS 252 = [0xF9, 0x00, 0x00] := by decide +kernel
theorem svec_bijou64_encodeS_13 : bijou64.encodeS (2 ^ 63 - 1)
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x06] := by decide +kernel
theorem svec_bijou64_encodeS_14 : bijou64.encodeS (-(2 ^ 63))
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07] := by decide +kernel

/-! ### Signed decode round-trips the vectors -/

theorem svec_bijou64_decodeS_1 : bijou64.decodeS [0x00] = .ok (0, 1) := by decide +kernel
theorem svec_bijou64_decodeS_2 : bijou64.decodeS [0x01] = .ok (-1, 1) := by decide +kernel
theorem svec_bijou64_decodeS_3 : bijou64.decodeS [0xF8, 0x01] = .ok (-125, 2) := by decide +kernel
theorem svec_bijou64_decodeS_4 : bijou64.decodeS [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x07]
  = .ok (-(2 ^ 63), 9) := by decide +kernel

/-! ### bijou64s error vectors -/

theorem svec_bijou64_decodeS_5 : bijou64.decodeS [] = .error .bufferTooShort := by decide +kernel
theorem svec_bijou64_decodeS_6 : bijou64.decodeS [0xF8] = .error .bufferTooShort := by decide +kernel
theorem svec_bijou64_decodeS_7 : bijou64.decodeS [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
  = .error .overflow := by decide +kernel

/-! ## bijou32s test vectors (specs/bijou32s.md) -/

theorem svec_bijou32_encodeS_1 : bijou32.encodeS 0 = [0x00] := by decide +kernel
theorem svec_bijou32_encodeS_2 : bijou32.encodeS (-1) = [0x01] := by decide +kernel
theorem svec_bijou32_encodeS_3 : bijou32.encodeS 1 = [0x02] := by decide +kernel
theorem svec_bijou32_encodeS_4 : bijou32.encodeS 125 = [0xFA] := by decide +kernel
theorem svec_bijou32_encodeS_5 : bijou32.encodeS (-126) = [0xFB] := by decide +kernel
theorem svec_bijou32_encodeS_6 : bijou32.encodeS 126 = [0xFC, 0x00] := by decide +kernel
theorem svec_bijou32_encodeS_7 : bijou32.encodeS (-127) = [0xFC, 0x01] := by decide +kernel
theorem svec_bijou32_encodeS_8 : bijou32.encodeS 253 = [0xFC, 0xFE] := by decide +kernel
theorem svec_bijou32_encodeS_9 : bijou32.encodeS (-254) = [0xFC, 0xFF] := by decide +kernel
theorem svec_bijou32_encodeS_10 : bijou32.encodeS 254 = [0xFD, 0x00, 0x00] := by decide +kernel
theorem svec_bijou32_encodeS_11 : bijou32.encodeS (2 ^ 31 - 1) = [0xFF, 0xFE, 0xFE, 0xFE, 0x02] := by decide +kernel
theorem svec_bijou32_encodeS_12 : bijou32.encodeS (-(2 ^ 31)) = [0xFF, 0xFE, 0xFE, 0xFE, 0x03] := by decide +kernel

/-! ### bijou32s decode and error vectors -/

theorem svec_bijou32_decodeS_1 : bijou32.decodeS [0xFF, 0xFE, 0xFE, 0xFE, 0x03] = .ok (-(2 ^ 31), 5) := by decide +kernel
theorem svec_bijou32_decodeS_2 : bijou32.decodeS [] = .error .bufferTooShort := by decide +kernel
theorem svec_bijou32_decodeS_3 : bijou32.decodeS [0xFC] = .error .bufferTooShort := by decide +kernel
theorem svec_bijou32_decodeS_4 : bijou32.decodeS [0xFF, 0xFF, 0xFF, 0xFF, 0xFF] = .error .overflow := by decide +kernel

/-! ## bijou128s test vectors (specs/bijou128s.md) -/

theorem svec_bijou128_encodeS_1 : bijou128.encodeS 0 = [0x00] := by decide +kernel
theorem svec_bijou128_encodeS_2 : bijou128.encodeS (-1) = [0x01] := by decide +kernel
theorem svec_bijou128_encodeS_3 : bijou128.encodeS 1 = [0x02] := by decide +kernel
theorem svec_bijou128_encodeS_4 : bijou128.encodeS 119 = [0xEE] := by decide +kernel
theorem svec_bijou128_encodeS_5 : bijou128.encodeS (-120) = [0xEF] := by decide +kernel
theorem svec_bijou128_encodeS_6 : bijou128.encodeS 120 = [0xF0, 0x00] := by decide +kernel
theorem svec_bijou128_encodeS_7 : bijou128.encodeS (-121) = [0xF0, 0x01] := by decide +kernel
theorem svec_bijou128_encodeS_8 : bijou128.encodeS 247 = [0xF0, 0xFE] := by decide +kernel
theorem svec_bijou128_encodeS_9 : bijou128.encodeS (-248) = [0xF0, 0xFF] := by decide +kernel
theorem svec_bijou128_encodeS_10 : bijou128.encodeS 248 = [0xF1, 0x00, 0x00] := by decide +kernel
theorem svec_bijou128_encodeS_11 : bijou128.encodeS (2 ^ 127 - 1)
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0E] := by decide +kernel
theorem svec_bijou128_encodeS_12 : bijou128.encodeS (-(2 ^ 127))
  = [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F] := by decide +kernel

/-! ### bijou128s decode and error vectors -/

theorem svec_bijou128_decodeS_1 : bijou128.decodeS
    [0xFF, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE,
     0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0xFE, 0x0F] = .ok (-(2 ^ 127), 17) := by decide +kernel
theorem svec_bijou128_decodeS_2 : bijou128.decodeS [] = .error .bufferTooShort := by decide +kernel
theorem svec_bijou128_decodeS_3 : bijou128.decodeS [0xF0] = .error .bufferTooShort := by decide +kernel
theorem svec_bijou128_decodeS_4 : bijou128.decodeS (List.replicate 17 0xFF) = .error .overflow := by decide +kernel

/-! ## The signed tier table (bijou64s SPEC "Signed Tier Table")

Boundary values of each row: encoded length changes exactly at the
documented magnitudes. -/

theorem svec_bijou64_encodedLenS_1 : bijou64.encodedLenS (-124) = 1 := by decide +kernel
theorem svec_bijou64_encodedLenS_2 : bijou64.encodedLenS 123 = 1 := by decide +kernel
theorem svec_bijou64_encodedLenS_3 : bijou64.encodedLenS (-125) = 2 := by decide +kernel
theorem svec_bijou64_encodedLenS_4 : bijou64.encodedLenS 124 = 2 := by decide +kernel
theorem svec_bijou64_encodedLenS_5 : bijou64.encodedLenS (-252) = 2 := by decide +kernel
theorem svec_bijou64_encodedLenS_6 : bijou64.encodedLenS 251 = 2 := by decide +kernel
theorem svec_bijou64_encodedLenS_7 : bijou64.encodedLenS (-253) = 3 := by decide +kernel
theorem svec_bijou64_encodedLenS_8 : bijou64.encodedLenS 252 = 3 := by decide +kernel
theorem svec_bijou64_encodedLenS_9 : bijou64.encodedLenS (-33020) = 3 := by decide +kernel
theorem svec_bijou64_encodedLenS_10 : bijou64.encodedLenS 33019 = 3 := by decide +kernel
theorem svec_bijou64_encodedLenS_11 : bijou64.encodedLenS (-33021) = 4 := by decide +kernel
theorem svec_bijou64_encodedLenS_12 : bijou64.encodedLenS 33020 = 4 := by decide +kernel
theorem svec_bijou64_encodedLenS_13 : bijou64.encodedLenS (-8421628) = 4 := by decide +kernel
theorem svec_bijou64_encodedLenS_14 : bijou64.encodedLenS 8421627 = 4 := by decide +kernel
theorem svec_bijou64_encodedLenS_15 : bijou64.encodedLenS (-8421629) = 5 := by decide +kernel
theorem svec_bijou64_encodedLenS_16 : bijou64.encodedLenS 8421628 = 5 := by decide +kernel
theorem svec_bijou64_encodedLenS_17 : bijou64.encodedLenS (-2155905276) = 5 := by decide +kernel
theorem svec_bijou64_encodedLenS_18 : bijou64.encodedLenS 2155905275 = 5 := by decide +kernel
theorem svec_bijou64_encodedLenS_19 : bijou64.encodedLenS (-(2 ^ 63)) = 9 := by decide +kernel
theorem svec_bijou64_encodedLenS_20 : bijou64.encodedLenS (2 ^ 63 - 1) = 9 := by decide +kernel

end Bijou
