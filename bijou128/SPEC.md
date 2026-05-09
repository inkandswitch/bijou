# bijou128

> "Plurality must never be posited without necessity."
> — William of Ockham

## Authors

- [Brooklyn Zelenka]

## Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14] when, and only when, they appear in all capitals, as shown here.

## Name

bijou128 (**BIJ**ective **O**ffset **U128**) is pronounced /biːʒuː wʌn twɛnti eɪt/ ("bee-zoo-one-twenty-eight"). The name encodes the format's three defining properties: bijectivity (canonical by construction), per-tier offset addition (the mechanism that achieves it), and the `u128` value type. That "bijou" is also French for "small jewel" is a happy coincidence for a compact encoding.

# Abstract

bijou128 is a [bijective][bijective numeration] variable-length encoding for unsigned 128-bit integers. It encodes values into 1–17 bytes using tag-byte framing inherited from [VARU64], modified with per-tier offsets so that canonicality is structural rather than checked at runtime.

It is the 128-bit sibling of [bijou64]. Both formats share the same family of design choices — tag-byte framing, per-tier offsets, big-endian payloads, length-from-first-byte. The two structural differences for bijou128 are a lower tag threshold (240 vs 248) and a wider tier count (16 vs 8), letting all multi-byte tiers fit in a single tag byte without extended framing.

# Introduction

Many binary protocols need a compact way to encode integers that are usually small but occasionally large. Variable-length integer encodings (varints) solve this, but most designs treat canonicality as an afterthought — something enforced by a runtime check in the decoder rather than by the structure of the encoding itself.

bijou128 inherits the structural-canonicality argument from [bijou64]'s spec verbatim. There is exactly one way to represent each `u128` value. Each tier subtracts a different cumulative offset from the value before encoding the payload. If you attempt to encode a value in the wrong tier, the offset arithmetic produces a _different value_, which fails any round-trip or hash comparison immediately. There is no overlong encoding to reject because the tier ranges are disjoint by construction.

## Design Goals

bijou128 was designed to satisfy the same properties as [bijou64], extended to the 128-bit domain:

| Property                  | Description                                                                                                                                                                                                                           |
|---------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Canonical by construction | Every value has exactly one encoding, enforced structurally by the format itself — not by a runtime check that can be omitted.                                                                                                        |
| Big-endian byte order     | Payload bytes are big-endian so that lexicographic byte comparison equals numeric comparison.                                                                                                                                         |
| Length from first byte    | The total encoding length is determined by inspecting only the first byte. All 16 multi-byte tiers fit in tag bytes `0xF0..=0xFF`, so no extended framing is required.                                                                |
| Compact for small values  | Values that fit in one byte (0–239) encode as that single byte with no overhead.                                                                                                                                                      |
| Full `u128` range         | The encoding covers all values from 0 to $2^{128} − 1$.                                                                                                                                                                               |
| Simple to implement       | The encoding and decoding algorithms are implementable in under 50 lines, in any language, with no dependencies or clever bit-shifting tricks.                                                                                        |
| Debuggable in a hexdump   | For single-byte values (the common case), the encoded byte is the value itself. For multi-byte values, the payload is contiguous big-endian bytes readable with minimal mental arithmetic.                                            |

# Format

bijou128 encodes unsigned 128-bit integers into 1–17 bytes. The encoding is a bijection: every `u128` value maps to exactly one byte sequence, and every valid byte sequence maps to exactly one `u128` value.

## Tag Byte

The first byte of an encoding is the _tag byte_. Its value determines how many additional bytes follow:

| First byte      | Total length | Tier | Offset (hex)                        |
|-----------------|--------------|------|-------------------------------------|
| `0x00` – `0xEF` | 1            | 0    | 0                                   |
| `0xF0`          | 2            | 1    | `0xF0`                              |
| `0xF1`          | 3            | 2    | `0x1F0`                             |
| `0xF2`          | 4            | 3    | `0x101F0`                           |
| `0xF3`          | 5            | 4    | `0x10101F0`                         |
| `0xF4`          | 6            | 5    | `0x1010101F0`                       |
| `0xF5`          | 7            | 6    | `0x101010101F0`                     |
| `0xF6`          | 8            | 7    | `0x10101010101F0`                   |
| `0xF7`          | 9            | 8    | `0x1010101010101F0`                 |
| `0xF8`          | 10           | 9    | `0x101010101010101F0`               |
| `0xF9`          | 11           | 10   | `0x10101010101010101F0`             |
| `0xFA`          | 12           | 11   | `0x1010101010101010101F0`           |
| `0xFB`          | 13           | 12   | `0x101010101010101010101F0`         |
| `0xFC`          | 14           | 13   | `0x10101010101010101010101F0`       |
| `0xFD`          | 15           | 14   | `0x1010101010101010101010101F0`     |
| `0xFE`          | 16           | 15   | `0x101010101010101010101010101F0`   |
| `0xFF`          | 17           | 16   | `0x10101010101010101010101010101F0` |

If the tag byte is below 240 (`0xF0`), the byte _is_ the encoded value and there are no additional bytes.

If the tag byte is 240 or above, let `tier = tag - 239` (giving tiers 1–16). The following `tier` bytes are the big-endian representation of a _payload_. The decoded value is:

```
value = OFFSET[tier] + payload
```

where the per-tier offsets are defined below.

## Offset Table

Each tier's offset is the first value not representable by any previous tier. The recurrence is:

```
OFFSET[0] = 0
OFFSET[1] = 240
OFFSET[n] = OFFSET[n-1] + 256^(n-1)    for n >= 2
```

Each OFFSET[n] for n ≥ 1 has a recognisable hex staircase: `(n-1)` repetitions of `01` prepended to `F0`. For n = 16, the offset is exactly 32 hex digits — fitting precisely within `u128`.

## Encoding

To encode a value `v`:

1. If `v < 240`, emit a single byte with value `v`.
2. Otherwise, find the tier `t` (1–16) such that `OFFSET[t] <= v < OFFSET[t+1]` (with `OFFSET[17]` treated as $2^{128}$).
3. Emit tag byte `239 + t`.
4. Emit `v - OFFSET[t]` as a `t`-byte big-endian integer.

### Worked Example

Encoding the value **67,000**:

1. 67,000 ≥ 240, so it is not a single-byte value.
2. Find the tier: `OFFSET[3] = 66,032 ≤ 67,000 < 16,843,248 = OFFSET[4]`, so tier = 3.
3. Tag byte: `239 + 3 = 242` → emit `0xF2`.
4. Payload: `67,000 − 66,032 = 968` → emit as 3-byte big-endian `0x00 0x03 0xC8`.
5. Result: `F2 00 03 C8` (4 bytes).

## Decoding

To decode from a byte buffer:

1. Read the tag byte. If the buffer is empty, the decoder MUST signal an error.
2. If `tag < 240`, the decoded value is `tag`. Consume 1 byte.
3. Otherwise, let `tier = tag - 239`. Read `tier` additional bytes. If fewer than `tier` bytes remain, the decoder MUST signal a buffer-too-short error.
4. Interpret the additional bytes as a big-endian unsigned integer (the _payload_).
5. Compute `value = OFFSET[tier] + payload`. If this addition overflows `u128` (possible only at tier 16), the decoder MUST signal an overflow error.
6. The decoded value is `value`. Consume `1 + tier` bytes total.

## Canonicality

bijou128 achieves canonicality _structurally_ (by construction) rather than by runtime rejection of overlong encodings. The argument is identical to [bijou64]'s — see that spec for full rationale.

A conforming decoder MUST check for exactly two error conditions:

1. **Buffer too short**: the input buffer contains fewer bytes than the tag byte requires.
2. **Overflow**: at tier 16, `OFFSET[16] + payload` exceeds $2^{128} - 1$.

No other validation is required.

# Properties

- The encoding length is determined entirely by the first byte.
- Encodings sort in the same order as the values they represent (lexicographic byte order equals numeric order).
- Values 0–239 are encoded as a single byte equal to the value.
- Maximum encoding length is 17 bytes (for values near `u128::MAX`).
- The format is _not_ wire-compatible with [bijou64]: the tag threshold differs (240 vs 248), so a value's encoding differs between the two crates.

# Test Vectors

Implementations SHOULD use these vectors to verify encoding compatibility.

| Value                                       | Encoded bytes (hex)                                         |
|---------------------------------------------|-------------------------------------------------------------|
| 0                                           | `00`                                                        |
| 1                                           | `01`                                                        |
| 42                                          | `2A`                                                        |
| 239                                         | `EF`                                                        |
| 240                                         | `F0 00`                                                     |
| 300                                         | `F0 3C`                                                     |
| 495                                         | `F0 FF`                                                     |
| 496                                         | `F1 00 00`                                                  |
| 1,000                                       | `F1 01 F8`                                                  |
| 65,535                                      | `F1 FE 0F`                                                  |
| 66,031                                      | `F1 FF FF`                                                  |
| 66,032                                      | `F2 00 00 00`                                               |
| 67,000                                      | `F2 00 03 C8`                                               |
| 16,843,247                                  | `F2 FF FF FF`                                               |
| 16,843,248                                  | `F3 00 00 00 00`                                            |
| 4,311,810,543                               | `F3 FF FF FF FF`                                            |
| OFFSET[8] (0x101_0101_0101_01F0)            | `F7 00 00 00 00 00 00 00 00`                                |
| OFFSET[16]                                  | `FF 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00`        |
| u128::MAX                                   | `FF FE FE FE FE FE FE FE FE FE FE FE FE FE FE FE 0F`        |

## Error Test Vectors

| Input bytes (hex)                                              | Expected error   | Rationale                                       |
|----------------------------------------------------------------|------------------|-------------------------------------------------|
| _(empty)_                                                      | Buffer too short | No tag byte present                             |
| `F1 00`                                                        | Buffer too short | Tag `F1` requires 2 payload bytes, only 1 given |
| `FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF`           | Overflow         | `OFFSET[16]` + `0xFF..FF` exceeds `u128::MAX`   |

# Prior Art

bijou128 inherits the prior-art discussion from [bijou64] verbatim — the design constraints (canonicality, big-endian, length-from-first-byte, debuggable hexdumps) and the alternatives considered ([LEB128], [vu128]/[vu64], [SQLite4's varint], [Git's pack offset encoding], [VARU64]) apply identically to the 128-bit variant. See [bijou64's spec][bijou64-prior-art] for the comparative analysis.

The relationship between bijou64 and bijou128 deserves a separate note. They share format family but are not interchangeable on the wire. A `u128` value that fits in `u64` (e.g., 300) encodes differently in the two formats:

- bijou64: `F8 34` (tag 248, payload `300 − 248 = 52`)
- bijou128: `F0 3C` (tag 240, payload `300 − 240 = 60`)

This is intentional. Picking the right crate at design time avoids the runtime cost of disambiguating which format a buffer was written with. Protocols using bijou128 SHOULD declare their use explicitly; mixed buffers SHOULD use a separate framing layer to indicate which format applies.

# License

This specification is adapted from [bijou64]'s specification, itself adapted from the [VARU64] specification by [Aljoscha Meyer], licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). The tag-byte framing and tier structure are inherited from VARU64; the per-tier offset addition (inspired by [Git's pack offset encoding] and [SQLite4's varint]) and surrounding specification text follow [bijou64]'s design.

This specification is licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

<!-- Links -->

[Aljoscha Meyer]: https://aljoscha-meyer.de/
[BCP 14]: https://www.rfc-editor.org/info/bcp14
[Brooklyn Zelenka]: https://github.com/expede
[Git's pack offset encoding]: https://git-scm.com/docs/pack-format#_original_version_1_pack_idx_files_have_the_following_format
[LEB128]: https://en.wikipedia.org/wiki/LEB128
[SQLite4's varint]: https://www.sqlite.org/src4/doc/trunk/www/varint.wiki
[VARU64]: https://github.com/AljoschaMeyer/varu64-rs
[bijective numeration]: https://en.wikipedia.org/wiki/Bijective_numeration
[bijou64]: ../bijou64/SPEC.md
[bijou64-prior-art]: ../bijou64/SPEC.md#prior-art
[vu128]: https://crates.io/crates/vu128
[vu64]: https://crates.io/crates/vu64
