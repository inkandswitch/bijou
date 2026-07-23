# bijou128

> "Plurality must never be posited without necessity."
>
> — William of Ockham

## Authors

- [Brooklyn Zelenka]

## Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14] when, and only when, they appear in all capitals, as shown here.

## Name

bijou128 (**BIJ**ective **O**ffset **U128**) is pronounced /biːʒuː wʌn ˈtwɛnti eɪt/ ("bee-zoo-one-twenty-eight"). The name encodes the format's three defining properties: bijectivity (canonical by construction), per-tier offset addition (the mechanism that achieves it), and the `u128` value type.

# Abstract

bijou128 is a [bijective][bijective numeration] variable-length encoding for unsigned 128-bit integers. It is the wider sibling of [bijou64][bijou64 SPEC]: same recurrence, same big-endian payload layout, same canonical-by-construction property — extended to cover the full `u128` range with 16 multi-byte tiers (1–17 bytes per value).

# Introduction

When a value domain exceeds 64 bits — UUIDs, content-addressed hashes truncated to 128 bits, large counters in distributed systems — a varint encoding for `u128` becomes useful. bijou128 is the natural extension of the bijou family to 128-bit values, preserving every structural property of [bijou64] while widening the integer range.

bijou128 inherits its design from [bijou64], differing in three structural ways:

1. **Narrower tag-byte threshold (240 vs 248).** Spanning 128 bits requires 16 multi-byte tiers (16 bytes of payload at most), which uses all of `0xF0..=0xFF` as tag values. This shrinks the single-byte tier from 0–247 to 0–239 — eight values reassigned from "literal" to "tag".
2. **Longer offset table.** 16 multi-byte offsets are defined (vs 8 in bijou64).
3. **Wider integer type.** Values are `u128` instead of `u64`; payloads are at most 16 bytes; the maximum encoding length is 17 bytes.

In every other respect — the offset recurrence, the encode and decode algorithms, the bijectivity property, the big-endian byte order, the length-from-first-byte property — bijou128 is identical to bijou64. See the [bijou64 specification][bijou64 SPEC] for the underlying design rationale.

## Design Goals

bijou128 was designed to satisfy the same properties as [bijou64], widened to the `u128` range. See the bijou64 specification for the full discussion. The property worth noting separately is:

| Property                          | Description                                                                                                                                                                                                                                       |
|-----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Full `u128` range                 | The encoding covers all values from 0 to $2^{128} − 1$. Because no platform integer type is wider than 128 bits in standard usage, this is the largest fixed-width varint the bijou family defines.                                               |
| JS boundary uses `bigint`         | `u128::MAX` exceeds JavaScript's `Number.MAX_SAFE_INTEGER` ($2^{53} - 1$) by 75 orders of magnitude. Wasm-bindgen marshals `u128` ↔ `bigint`, so bijou128 implementations targeting JS MUST use `bigint` at the boundary, not `Number`. |

## Relationship to bijou64

bijou128 is **not wire-compatible** with bijou64. The two formats use different tag thresholds and therefore decode the same byte sequence differently:

- A single byte `0xF4` decodes as **value 244** in bijou64 (tier 0, byte = value).
- The same byte `0xF4` decodes as **tier 5 tag** in bijou128, requiring 5 more payload bytes.

Picking the right variant is a deployment decision, not a runtime concern. If your protocol's value domain is `u64`, use bijou64; if it's `u128`, use bijou128. Don't mix encodings on the same wire without an out-of-band signal (such as a separate version byte).

# Format

bijou128 encodes unsigned 128-bit integers into 1–17 bytes. The encoding is a bijection: every `u128` value maps to exactly one byte sequence, and every valid byte sequence maps to exactly one `u128` value.

## Tag Byte

The first byte of an encoding is the _tag byte_. Its value determines how many additional bytes follow:

| First byte      | Total length | Offset (decimal)                              |
|-----------------|--------------|-----------------------------------------------|
| `0x00` – `0xEF` | 1            | 0                                             |
| `0xF0`          | 2            | 240                                           |
| `0xF1`          | 3            | 496                                           |
| `0xF2`          | 4            | 66,032                                        |
| `0xF3`          | 5            | 16,843,248                                    |
| `0xF4`          | 6            | 4,311,810,544                                 |
| `0xF5`          | 7            | 1,103,823,438,320                             |
| `0xF6`          | 8            | 282,578,800,148,976                           |
| `0xF7`          | 9            | 72,340,172,838,076,912                        |
| `0xF8`          | 10           | 18,519,084,246,547,628,528                    |
| `0xF9`          | 11           | 4,740,885,567,116,192,842,224                 |
| `0xFA`          | 12           | 1,213,666,705,181,745,367,548,400             |
| `0xFB`          | 13           | 310,698,676,526,526,814,092,329,456           |
| `0xFC`          | 14           | 79,538,861,190,790,864,407,636,279,792        |
| `0xFD`          | 15           | 20,361,948,464,842,461,288,354,887,565,808    |
| `0xFE`          | 16           | 5,212,658,806,999,670,089,818,851,216,785,904 |
| `0xFF`          | 17           | 1,334,440,654,591,915,542,993,625,911,497,130,480 |

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

Giving the concrete offset table:

| Tier | Tag    | Offset (hex)                          |
|------|--------|---------------------------------------|
| 0    | —      | `0x0`                                 |
| 1    | `0xF0` | `0xF0`                                |
| 2    | `0xF1` | `0x1F0`                               |
| 3    | `0xF2` | `0x101F0`                             |
| 4    | `0xF3` | `0x10101F0`                           |
| 5    | `0xF4` | `0x1010101F0`                         |
| 6    | `0xF5` | `0x101010101F0`                       |
| 7    | `0xF6` | `0x10101010101F0`                     |
| 8    | `0xF7` | `0x1010101010101F0`                   |
| 9    | `0xF8` | `0x101010101010101F0`                 |
| 10   | `0xF9` | `0x10101010101010101F0`               |
| 11   | `0xFA` | `0x1010101010101010101F0`             |
| 12   | `0xFB` | `0x101010101010101010101F0`           |
| 13   | `0xFC` | `0x10101010101010101010101F0`         |
| 14   | `0xFD` | `0x1010101010101010101010101F0`       |
| 15   | `0xFE` | `0x101010101010101010101010101F0`     |
| 16   | `0xFF` | `0x10101010101010101010101010101F0`   |

The hex column shows the same staircase pattern as bijou64: each offset ends with `0xF0` (240, the tier 0 capacity) and prepends one `01` byte per tier. The pattern simply extends further than in bijou64.

## Encoding

To encode a value `v`:

1. If `v < 240`, emit a single byte with value `v`.
2. Otherwise, find the tier `t` (1–16) such that `OFFSET[t] <= v < OFFSET[t+1]` (with `OFFSET[17]` treated as $2^{128}$).
3. Emit tag byte `239 + t`.
4. Emit `v - OFFSET[t]` as a `t`-byte big-endian integer.

### Worked Example

There are two separate subtractions in the encoder, and it is important not to confuse them:

- **Tag byte**: always `239 + tier`. The constant 239 maps between the tier number (1–16) and the tag byte (`0xF0`–`0xFF`). This is the same for every multi-byte tier.

- **Payload**: always `value - OFFSET[tier]`. The offset is _different_ for each tier — it is the cumulative count of values representable by all previous tiers. This subtraction is what makes the encoding bijective.

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

> "The best error message is the one that never shows up."
>
> — Thomas Fuchs

bijou128 achieves canonicality _structurally_ (by construction) rather than by runtime rejection of overlong encodings. See the [bijou64 specification][bijou64 SPEC] for the full discussion; the same arguments apply here verbatim, with tiers numbered 1–16 instead of 1–8.

### Minimal Decoder Obligations

A conforming decoder MUST check for exactly two error conditions:

1. Buffer too short (not enough bytes for the tier).
2. Arithmetic overflow on tier 16 (`OFFSET[16] + payload > u128::MAX`).

No other validation is required. In particular, there is no "non-canonical encoding" error because non-canonical encodings are structurally impossible.

## Error Conditions

A conforming decoder MUST signal an error for:

1. **Buffer too short**: the input buffer contains fewer bytes than the tag byte requires.
2. **Overflow**: at tier 16, `OFFSET[16] + payload` exceeds $2^{128} - 1$.

No other error conditions exist.

# Properties

- The encoding length is determined entirely by the first byte.
- Encodings sort in the same order as the values they represent (lexicographic byte order equals numeric order).
- Values 0–239 are encoded as a single byte equal to the value.
- Maximum encoding length is 17 bytes (for values near `u128::MAX`).
- The single-byte tier is _narrower_ than bijou64's (0–239 vs 0–247) because bijou128 reserves more tag values for tier framing.

# Test Vectors

Implementations SHOULD use these vectors to verify encoding compatibility.

| Value                                     | Encoded bytes (hex)                                       |
|-------------------------------------------|-----------------------------------------------------------|
| 0                                         | `00`                                                      |
| 1                                         | `01`                                                      |
| 42                                        | `2A`                                                      |
| 239                                       | `EF`                                                      |
| 240                                       | `F0 00`                                                   |
| 241                                       | `F0 01`                                                   |
| 495                                       | `F0 FF`                                                   |
| 496                                       | `F1 00 00`                                                |
| 65,535                                    | `F1 FE 0F`                                                |
| 66,031                                    | `F1 FF FF`                                                |
| 66,032                                    | `F2 00 00 00`                                             |
| 67,000                                    | `F2 00 03 C8`                                             |
| 16,843,247                                | `F2 FF FF FF`                                             |
| 16,843,248                                | `F3 00 00 00 00`                                          |
| 2^32 - 1                                  | `F3 FE FE FE 0F`                                          |
| 2^64 - 1                                  | `F7 FE FE FE FE FE FE FE 0F`                              |
| 2^128 - 1                                 | `FF FE FE FE FE FE FE FE FE FE FE FE FE FE FE FE 0F`      |

## Error Test Vectors

| Input bytes (hex)                                       | Expected error   | Rationale                                          |
|---------------------------------------------------------|------------------|----------------------------------------------------|
| _(empty)_                                               | Buffer too short | No tag byte present                                |
| `F1 00`                                                 | Buffer too short | Tag `F1` requires 2 payload bytes, only 1 provided |
| `FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF`    | Overflow         | `OFFSET[16]` + `0xFF..FF` exceeds `u128::MAX`      |

# Comparison with bijou32 and bijou64

| Property         | bijou32       | bijou64       | bijou128      |
|------------------|---------------|---------------|---------------|
| Integer type     | `u32`         | `u64`         | `u128`        |
| Max bytes        | 5             | 9             | 17            |
| Tag threshold    | 252           | 248           | 240           |
| Multi-byte tags  | `0xFC`–`0xFF` | `0xF8`–`0xFF` | `0xF0`–`0xFF` |
| Multi-byte tiers | 4             | 8             | 16            |
| Tier 0 width     | 0–251         | 0–247         | 0–239         |

Each variant uses the widest tag threshold its tier count allows. This maximises the size of the single-byte tier — the most common case for many workloads — at the cost of wire-incompatibility between widths.

# Prior Art

bijou128 inherits everything from [bijou64][bijou64 SPEC], which in turn builds on [VARU64], [LEB128], [SQLite4's varint], and [Git's pack offset encoding]. See the bijou64 specification's "Prior Art" section for the full discussion.

The bijou64 specification anticipates a 128-bit variant in its "Future Extensions" section but speculates that bijou128 "would require tag bytes beyond `0xFF`, which would need an extended framing scheme (e.g., `0xFF` followed by a secondary length byte)." That speculation turns out to be unnecessary: by narrowing the tag threshold from 248 to 240, all 16 multi-byte tiers fit in the existing `0xF0..=0xFF` tag range with no secondary length byte. The cost is eight values (240–247) that are no longer single-byte literals — a small trade for staying within a single tag byte.

# License

This specification is adapted from the [VARU64] specification by [Aljoscha Meyer], licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), and from the [bijou64 specification][bijou64 SPEC] by [Brooklyn Zelenka]. The tag-byte framing and offset recurrence are inherited from bijou64; the threshold-240 framing, the 16-tier offset table, and surrounding specification text are new in bijou128.

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
[bijou64]: ../bijou64
[bijou64 SPEC]: ./bijou64.md
