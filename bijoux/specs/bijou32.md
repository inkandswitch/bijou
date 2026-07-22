# bijou32

> "Plurality must never be posited without necessity."
>
> — William of Ockham

## Authors

- [Brooklyn Zelenka]

## Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14] when, and only when, they appear in all capitals, as shown here.

## Name

bijou32 (**BIJ**ective **O**ffset **U32**) is pronounced /biːʒuː ˈθɜːti tuː/ ("bee-zoo-thirty-two"). The name encodes the format's three defining properties: bijectivity (canonical by construction), per-tier offset addition (the mechanism that achieves it), and the `u32` value type.

# Abstract

bijou32 is a [bijective][bijective numeration] variable-length encoding for unsigned 32-bit integers. It is the narrower sibling of [bijou64][bijou64 SPEC]: same recurrence, same big-endian payload layout, same canonical-by-construction property — sized for the `u32` value range with 4 multi-byte tiers (1–5 bytes per value) and a wider single-byte tier (0–251) since fewer tag values need to be reserved for tier framing.

# Introduction

Many binary protocols need a compact way to encode integers that are usually small but occasionally large. bijou32 is the narrowest member of the bijou family — when the value domain is known to fit in `u32`, bijou32 gives the smallest encoding for values up to 251 (one byte) and the smallest maximum size (5 bytes for `u32::MAX`).

bijou32 inherits its design from [bijou64], differing in three structural ways:

1. **Wider tag-byte threshold (252 vs 248).** Because only 4 multi-byte tiers are needed to span 32 bits, the tier framing uses only tags `0xFC..=0xFF`. The remaining four tag positions (`0xF8..=0xFB`) are absorbed into the single-byte tier, extending it to 0–251 instead of 0–247.
2. **Shorter offset table.** Only 4 multi-byte offsets are defined (vs 8 in bijou64).
3. **Narrower integer type.** Values are `u32` instead of `u64`; payloads are at most 4 bytes; the maximum encoding length is 5 bytes.

In every other respect — the offset recurrence, the encode and decode algorithms, the bijectivity property, the big-endian byte order, the length-from-first-byte property — bijou32 is identical to bijou64. See the [bijou64 specification][bijou64 SPEC] for the underlying design rationale.

## Design Goals

bijou32 was designed to satisfy the same properties as [bijou64], narrowed to the `u32` range. See the bijou64 specification for the full discussion. The property worth noting separately is:

| Property                  | Description                                                                                                                                                                                  |
|---------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Full `u32` range          | The encoding covers all values from 0 to $2^{32} − 1$. Because `u32::MAX < 2^{53}` (the JavaScript safe-integer boundary), bijou32 can cross the wasm/JS boundary as a plain `Number`, not a `BigInt`. |

## Relationship to bijou64

bijou32 is **not wire-compatible** with bijou64. The two formats use different tag thresholds and therefore decode the same byte sequence differently:

- A single byte `0xFA` decodes as **value 250** in bijou32 (tier 0, byte = value).
- The same byte `0xFA` decodes as **tier 3 tag** in bijou64, requiring 3 more payload bytes.

Picking the right variant is a deployment decision, not a runtime concern. If your protocol's value domain is `u32`, use bijou32; if it's `u64`, use bijou64. Don't mix encodings on the same wire without an out-of-band signal (such as a separate version byte).

# Format

bijou32 encodes unsigned 32-bit integers into 1–5 bytes. The encoding is a bijection: every `u32` value maps to exactly one byte sequence, and every valid byte sequence maps to exactly one `u32` value.

## Tag Byte

The first byte of an encoding is the _tag byte_. Its value determines how many additional bytes follow:

| First byte      | Total length | Offset (decimal) | Offset (hex) |
|-----------------|--------------|------------------|--------------|
| `0x00` – `0xFB` | 1            | 0                | `0x00`       |
| `0xFC`          | 2            | 252              | `0xFC`       |
| `0xFD`          | 3            | 508              | `0x1FC`      |
| `0xFE`          | 4            | 66,044           | `0x101FC`    |
| `0xFF`          | 5            | 16,843,260       | `0x10101FC`  |

If the tag byte is below 252 (`0xFC`), the byte _is_ the encoded value and there are no additional bytes.

If the tag byte is 252 or above, let `tier = tag - 251` (giving tiers 1–4). The following `tier` bytes are the big-endian representation of a _payload_. The decoded value is:

```
value = OFFSET[tier] + payload
```

where the per-tier offsets are defined below.

## Offset Table

Each tier's offset is the first value not representable by any previous tier. The recurrence is:

```
OFFSET[0] = 0
OFFSET[1] = 252
OFFSET[n] = OFFSET[n-1] + 256^(n-1)    for n >= 2
```

Giving the concrete values:

| Tier | Tag    | Offset       | Start        | End (inclusive) |
|------|--------|--------------|--------------|-----------------|
| 0    | —      | `0x00`       | `0x00`       | `0xFB`          |
| 1    | `0xFC` | `0xFC`       | `0xFC`       | `0x1FB`         |
| 2    | `0xFD` | `0x1FC`      | `0x1FC`      | `0x101FB`       |
| 3    | `0xFE` | `0x101FC`    | `0x101FC`    | `0x10101FB`     |
| 4    | `0xFF` | `0x10101FC`  | `0x10101FC`  | `0xFFFFFFFF`    |

The recurrence is identical to bijou64's; only the base case (`OFFSET[1] = 252` instead of 248) differs. This shifts every offset by 4, but otherwise preserves the staircase structure.

## Encoding

To encode a value `v`:

1. If `v < 252`, emit a single byte with value `v`.
2. Otherwise, find the tier `t` (1–4) such that `OFFSET[t] <= v < OFFSET[t+1]` (with `OFFSET[5]` treated as $2^{32}$).
3. Emit tag byte `251 + t`.
4. Emit `v - OFFSET[t]` as a `t`-byte big-endian integer.

### Worked Example

There are two separate subtractions in the encoder, and it is important not to confuse them:

- **Tag byte**: always `251 + tier`. The constant 251 maps between the tier number (1–4) and the tag byte (`0xFC`–`0xFF`). This is the same for every multi-byte tier.

- **Payload**: always `value - OFFSET[tier]`. The offset is _different_ for each tier — it is the cumulative count of values representable by all previous tiers. This subtraction is what makes the encoding bijective.

Encoding the value **67,000**:

1. 67,000 ≥ 252, so it is not a single-byte value.
2. Find the tier: `OFFSET[3] = 66,044 ≤ 67,000 < 16,843,260 = OFFSET[4]`, so tier = 3.
3. Tag byte: `251 + 3 = 254` → emit `0xFE`.
4. Payload: `67,000 − 66,044 = 956` → emit as 3-byte big-endian `0x00 0x03 0xBC`.
5. Result: `FE 00 03 BC` (4 bytes).

| Tier | Offset (decimal) | Offset (hex)  |
|------|------------------|---------------|
| 1    | 252              | `0xFC`        |
| 2    | 508              | `0x1FC`       |
| 3    | 66,044           | `0x101FC`     |
| 4    | 16,843,260       | `0x10101FC`   |

The hex column shows the same staircase pattern as bijou64, shifted to end in `0xFC` (252) instead of `0xF8` (248).

## Decoding

To decode from a byte buffer:

1. Read the tag byte. If the buffer is empty, the decoder MUST signal an error.
2. If `tag < 252`, the decoded value is `tag`. Consume 1 byte.
3. Otherwise, let `tier = tag - 251`. Read `tier` additional bytes. If fewer than `tier` bytes remain, the decoder MUST signal a buffer-too-short error.
4. Interpret the additional bytes as a big-endian unsigned integer (the _payload_).
5. Compute `value = OFFSET[tier] + payload`. If this addition overflows `u32` (possible only at tier 4), the decoder MUST signal an overflow error.
6. The decoded value is `value`. Consume `1 + tier` bytes total.

## Canonicality

> "The best error message is the one that never shows up."
>
> — Thomas Fuchs

bijou32 achieves canonicality _structurally_ (by construction) rather than by runtime rejection of overlong encodings. See the [bijou64 specification][bijou64 SPEC] for the full discussion; the same arguments apply here verbatim, with tiers numbered 1–4 instead of 1–8.

### Minimal Decoder Obligations

A conforming decoder MUST check for exactly two error conditions:

1. Buffer too short (not enough bytes for the tier).
2. Arithmetic overflow on tier 4 (`OFFSET[4] + payload > u32::MAX`).

No other validation is required. In particular, there is no "non-canonical encoding" error because non-canonical encodings are structurally impossible.

## Error Conditions

A conforming decoder MUST signal an error for:

1. **Buffer too short**: the input buffer contains fewer bytes than the tag byte requires.
2. **Overflow**: at tier 4, `OFFSET[4] + payload` exceeds $2^{32} - 1$.

No other error conditions exist.

# Properties

- The encoding length is determined entirely by the first byte.
- Encodings sort in the same order as the values they represent (lexicographic byte order equals numeric order).
- Values 0–251 are encoded as a single byte equal to the value.
- Maximum encoding length is 5 bytes (for values near `u32::MAX`).
- The single-byte tier is _wider_ than bijou64's (0–251 vs 0–247) because bijou32 reserves fewer tag values for tier framing.

# Test Vectors

Implementations SHOULD use these vectors to verify encoding compatibility.

| Value         | Encoded bytes (hex) |
|---------------|---------------------|
| 0             | `00`                |
| 1             | `01`                |
| 42            | `2A`                |
| 247           | `F7`                |
| 251           | `FB`                |
| 252           | `FC 00`             |
| 300           | `FC 30`             |
| 507           | `FC FF`             |
| 508           | `FD 00 00`          |
| 1,000         | `FD 01 EC`          |
| 65,535        | `FD FE 03`          |
| 66,043        | `FD FF FF`          |
| 66,044        | `FE 00 00 00`       |
| 67,000        | `FE 00 03 BC`       |
| 16,843,259    | `FE FF FF FF`       |
| 16,843,260    | `FF 00 00 00 00`    |
| 4,294,967,295 | `FF FE FE FE 03`    |

## Error Test Vectors

| Input bytes (hex) | Expected error   | Rationale                                          |
|-------------------|------------------|----------------------------------------------------|
| _(empty)_         | Buffer too short | No tag byte present                                |
| `FD 00`           | Buffer too short | Tag `FD` requires 2 payload bytes, only 1 provided |
| `FF FF FF FF FF`  | Overflow         | `OFFSET[4]` + `0xFFFFFFFF` exceeds `u32::MAX`      |

# Comparison with bijou64 and bijou128

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

bijou32 inherits everything from [bijou64][bijou64 SPEC], which in turn builds on [VARU64], [LEB128], [SQLite4's varint], and [Git's pack offset encoding]. See the bijou64 specification's "Prior Art" section for the full discussion. The only design choice unique to bijou32 is the wider tag threshold, which has no antecedent in the cited literature — VARU64 picked 248 because it needs 8 multi-byte tags, and the choice cascades down.

# License

This specification is adapted from the [VARU64] specification by [Aljoscha Meyer], licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), and from the [bijou64 specification][bijou64 SPEC] by [Brooklyn Zelenka]. The tag-byte framing and tier structure are inherited from VARU64; the per-tier offset addition is inherited from bijou64; the threshold-252 framing and surrounding specification text are new in bijou32.

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
[bijou64 SPEC]: ../bijou64/SPEC.md
