# bijou64s

> ὁδὸς ἄνω κάτω μία καὶ ὡυτή
>
> "The road up and the road down are one and the same."
>
> — Heraclitus, fragment DK B60 (preserved in Hippolytus, _Refutation of All Heresies_ IX.10.4)

## Authors

- [Brooklyn Zelenka]

## Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14] when, and only when, they appear in all capitals, as shown here.

## Name

bijou64s (**BIJ**ective **O**ffset **U64**, **S**igned) is pronounced /biːʒuː sɪksti fɔːr ɛs/ ("bee-zoo-sixty-four-ess").

# Abstract

bijou64s is a bijective variable-length encoding for signed 64-bit integers. It is defined as the composition of the standard [zigzag] bijection with the [bijou64] wire format: every `i64` maps to exactly one 1–9 byte encoding and vice versa, small-magnitude values of either sign encode in one byte, and the first byte alone determines the total length.

# Introduction

Signed workloads — deltas, offsets, differences — are dominated by small magnitudes of both signs. A varint that treats the sign bit as part of a two's-complement value places small negatives at the far end of the unsigned range, costing maximum-length encodings for the most common values. bijou64s instead folds the sign into the least-significant bit before applying the unsigned format, so encoded length grows with magnitude regardless of sign.

## Design Goals

In priority order (and identical to bijou64 except the last):

1. **Bijectivity.** Every `i64` has exactly one encoding; every valid encoding has exactly one `i64`. Structural — no runtime canonicality checks.
2. **Length from the first byte.** Single-pass framing without lookahead.
3. **Density on common values.** One byte for the 248 values nearest zero.
4. ~~Lexicographic order matches numeric order~~ — **deliberately sacrificed** (see [Ordering](#ordering)).

# Format

## The Zigzag Map

A value `n : i64` is first mapped to `z : u64`:

```text
zigzag(n)   = (n << 1) XOR (n >> 63)     (arithmetic shift right)
unzigzag(z) = (z >> 1) XOR -(z AND 1)
```

This interleaves the signed integers around zero:

| `n`    | 0 | -1 | 1 | -2 | 2 | … | -124 | … | i64::MAX     | i64::MIN |
|--------|---|----|---|----|---|---|------|---|--------------|----------|
| `z`    | 0 | 1  | 2 | 3  | 4 | … | 247  | … | 2⁶⁴ − 2      | 2⁶⁴ − 1  |

`zigzag` is a bijection on the full 64-bit space, so composing it with the bijective bijou64 format preserves bijectivity end to end.

## The Wire Format

The mapped value `z` MUST be encoded exactly as specified by [bijou64] — the same tag byte scheme, offset table, big-endian payloads, and tier structure. An implementation of bijou64s is therefore:

```text
encode_s(n)   = encode_u(zigzag(n))
decode_s(b)   = unzigzag(decode_u(b))
```

No additional framing, flags, or bias constants exist.

## Signed Tier Table

Derived from the bijou64 tier table via `unzigzag`:

| Tag        | Total bytes | Signed range                                            |
|------------|-------------|---------------------------------------------------------|
| 0x00–0xF7  | 1           | −124 ..= 123                                            |
| 0xF8       | 2           | −252 ..= −125 and 124 ..= 251                           |
| 0xF9       | 3           | −33,020 ..= −253 and 252 ..= 33,019                     |
| 0xFA       | 4           | −8,421,628 ..= −33,021 and 33,020 ..= 8,421,627         |
| 0xFB       | 5           | −2,155,905,276 ..= −8,421,629 and 8,421,628 ..= 2,155,905,275 |
| 0xFC–0xFF  | 6–9         | (continue per the bijou64 offset table, split around 0) |
| 0xFF       | 9           | down to i64::MIN and up to i64::MAX                     |

## Ordering

Byte-lexicographic order of bijou64s encodings is **zigzag order**, not numeric order: `0, −1, 1, −2, 2, …` (by magnitude; the negative precedes the positive at equal magnitude). Consumers requiring memcomparable signed keys MUST NOT use bijou64s for that purpose.

## Errors

Exactly bijou64's: a buffer shorter than its tag byte requires is invalid (`BufferTooShort`), and a 9-byte encoding whose payload exceeds the `u64` range is invalid (`Overflow`). There are no signed-specific error conditions.

# Test Vectors

| Value                | Encoding (hex)                  |
|----------------------|---------------------------------|
| 0                    | `00`                            |
| −1                   | `01`                            |
| 1                    | `02`                            |
| −2                   | `03`                            |
| 2                    | `04`                            |
| 123                  | `F6`                            |
| −124                 | `F7`                            |
| 124                  | `F8 00`                         |
| −125                 | `F8 01`                         |
| 251                  | `F8 FE`                         |
| −252                 | `F8 FF`                         |
| 252                  | `F9 00 00`                      |
| i64::MAX             | `FF FE FE FE FE FE FE FE 06`    |
| i64::MIN             | `FF FE FE FE FE FE FE FE 07`    |

## Error Test Vectors

| Input (hex)                       | Error          |
|-----------------------------------|----------------|
| (empty)                           | BufferTooShort |
| `F8`                              | BufferTooShort |
| `FF FF FF FF FF FF FF FF FF`      | Overflow       |

# Prior Art

The zigzag map is the standard signed-integer mapping used by [Protocol Buffers] (`sint64`), Apache Avro, and Apache Thrift. bijou64s contributes the composition with a structurally canonical base format: unlike zigzag-over-LEB128, a bijou64s decoder never needs to reject overlong encodings, because none exist.

Rejected alternatives (benchmarked; see the repository's design notes): two's-complement casting (9-byte encodings for all small negatives), and a sign-in-tag "mirrored tier" layout (preserves numeric byte order but costs 2–6× on decode). The symmetric single-byte window was confirmed against 5.4M signed values from a real editing-trace workload; skewed windows offered < 0.5 % density improvement.

# License

This specification is licensed under [CC BY-SA 4.0].

[BCP 14]: https://www.rfc-editor.org/info/bcp14
[Brooklyn Zelenka]: https://github.com/expede
[CC BY-SA 4.0]: https://creativecommons.org/licenses/by-sa/4.0/
[Protocol Buffers]: https://protobuf.dev/programming-guides/encoding/#signed-ints
[bijou64]: ./bijou64.md
[zigzag]: https://protobuf.dev/programming-guides/encoding/#signed-ints
