# bijou128s

## Authors

- [Brooklyn Zelenka]

## Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14] when, and only when, they appear in all capitals, as shown here.

# Abstract

bijou128s is a bijective variable-length encoding for signed 128-bit integers: the standard [zigzag] bijection composed with the [bijou128] wire format. It is the 128-bit member of the signed bijou family; all design rationale, the zigzag definition, ordering caveats, and error semantics are specified in [bijou64s] and apply here with the widths substituted.

# Format

A value `n : i128` is mapped to `z : u128` by `zigzag(n) = (n << 1) XOR (n >> 127)` (arithmetic shift), and `z` MUST be encoded exactly as specified by [bijou128]. No additional framing, flags, or bias constants exist.

## Signed Tier Table

| Tag        | Total bytes | Signed range                                      |
|------------|-------------|---------------------------------------------------|
| 0x00–0xEF  | 1           | −120 ..= 119                                      |
| 0xF0       | 2           | −248 ..= −121 and 120 ..= 247                     |
| 0xF1       | 3           | −33,016 ..= −249 and 248 ..= 33,015               |
| 0xF2–0xFE  | 4–16        | (continue per the bijou128 offset table, split around 0) |
| 0xFF       | 17          | down to i128::MIN and up to i128::MAX             |

## Ordering

Byte-lexicographic order is **zigzag order, not numeric order**. See [bijou64s].

# Test Vectors

| Value      | Encoding (hex)                                       |
|------------|------------------------------------------------------|
| 0          | `00`                                                 |
| −1         | `01`                                                 |
| 1          | `02`                                                 |
| 119        | `EE`                                                 |
| −120       | `EF`                                                 |
| 120        | `F0 00`                                              |
| −121       | `F0 01`                                              |
| 247        | `F0 FE`                                              |
| −248       | `F0 FF`                                              |
| 248        | `F1 00 00`                                           |
| i128::MAX  | `FF FE FE FE FE FE FE FE FE FE FE FE FE FE FE FE 0E` |
| i128::MIN  | `FF FE FE FE FE FE FE FE FE FE FE FE FE FE FE FE 0F` |

## Error Test Vectors

| Input (hex)             | Error          |
|-------------------------|----------------|
| (empty)                 | BufferTooShort |
| `F0`                    | BufferTooShort |
| `FF FF … FF` (17 bytes) | Overflow       |

# License

This specification is licensed under [CC BY-SA 4.0].

[BCP 14]: https://www.rfc-editor.org/info/bcp14
[Brooklyn Zelenka]: https://github.com/expede
[CC BY-SA 4.0]: https://creativecommons.org/licenses/by-sa/4.0/
[bijou128]: ./bijou128.md
[bijou64s]: ./bijou64s.md
[zigzag]: https://protobuf.dev/programming-guides/encoding/#signed-ints
