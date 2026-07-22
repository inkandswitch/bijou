# bijou32s

## Authors

- [Brooklyn Zelenka]

## Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14] when, and only when, they appear in all capitals, as shown here.

# Abstract

bijou32s is a bijective variable-length encoding for signed 32-bit integers: the standard [zigzag] bijection composed with the [bijou32] wire format. It is the 32-bit member of the signed bijou family; all design rationale, the zigzag definition, ordering caveats, and error semantics are specified in [bijou64s] and apply here with the widths substituted.

# Format

A value `n : i32` is mapped to `z : u32` by `zigzag(n) = (n << 1) XOR (n >> 31)` (arithmetic shift), and `z` MUST be encoded exactly as specified by [bijou32]. No additional framing, flags, or bias constants exist.

## Signed Tier Table

| Tag        | Total bytes | Signed range                                    |
|------------|-------------|-------------------------------------------------|
| 0x00–0xFB  | 1           | −126 ..= 125                                    |
| 0xFC       | 2           | −254 ..= −127 and 126 ..= 253                   |
| 0xFD       | 3           | −33,022 ..= −255 and 254 ..= 33,021             |
| 0xFE       | 4           | −8,421,630 ..= −33,023 and 33,022 ..= 8,421,629 |
| 0xFF       | 5           | down to i32::MIN and up to i32::MAX             |

## Ordering

Byte-lexicographic order is **zigzag order, not numeric order**. See [bijou64s].

# Test Vectors

| Value     | Encoding (hex)    |
|-----------|-------------------|
| 0         | `00`              |
| −1        | `01`              |
| 1         | `02`              |
| 125       | `FA`              |
| −126      | `FB`              |
| 126       | `FC 00`           |
| −127      | `FC 01`           |
| 253       | `FC FE`           |
| −254      | `FC FF`           |
| 254       | `FD 00 00`        |
| i32::MAX  | `FF FE FE FE 02`  |
| i32::MIN  | `FF FE FE FE 03`  |

## Error Test Vectors

| Input (hex)       | Error          |
|-------------------|----------------|
| (empty)           | BufferTooShort |
| `FC`              | BufferTooShort |
| `FF FF FF FF FF`  | Overflow       |

# License

This specification is licensed under [CC BY-SA 4.0].

[BCP 14]: https://www.rfc-editor.org/info/bcp14
[Brooklyn Zelenka]: https://github.com/expede
[CC BY-SA 4.0]: https://creativecommons.org/licenses/by-sa/4.0/
[bijou32]: ./bijou32.md
[bijou64s]: ./bijou64s.md
[zigzag]: https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding
