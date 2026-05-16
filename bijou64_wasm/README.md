# bijou64_wasm

> [!WARNING]
> Early release preview. The API is unstable. Not for production use.

Wasm/JavaScript bindings for [`bijou64`](../bijou64) — a bijective,
length-prefixed varint encoding for `u64` (1–9 bytes per value, with
structural canonicality so every value has exactly one encoding).

## Installation

This crate is built into a universal NPM package via
[`wasm-bodge`](https://github.com/alexjg/wasm-bodge). From the workspace
dev shell (the `bodge` command is hard-wired to this crate — there is
only one wasm crate in the workspace today):

```sh
bodge
```

The output lands in `bijou64_wasm/dist/` and is publishable as-is
(`pnpm publish`).

## Usage

```js
import { encode, decode, decodeAll, encodedLen, MAX_BYTES } from "bijou64";

// Encode
const bytes = encode(300n);                    // Uint8Array([0xF8, 0x34])

// Decode a single value (returns { value, bytesRead })
const { value, bytesRead } = decode(bytes);    // 300n, 2

// Decode every value in a buffer (returns BigUint64Array)
const buf = new Uint8Array([...encode(42n), ...encode(300n), ...encode(65535n)]);
const values = decodeAll(buf);                 // BigUint64Array([42n, 300n, 65535n])

// Predict size without encoding
encodedLen((1n << 32n) + 5n);                  // 5
MAX_BYTES();                                   // 9
```

`u64` values cross the boundary as JS `bigint`. `bytesRead` is a regular
`number`. `decodeAll` returns a typed `BigUint64Array` rather than a
plain JS array of `bigint`s — denser and faster for large batches.

### Slim / manual init

If you ship `bijou64_wasm` from a downstream library, import from `/slim`
to avoid forcing a wasm-init strategy on your consumers — see the
[wasm-bodge README](https://github.com/alexjg/wasm-bodge#the-slim-escape-hatch)
for why.

```js
import { encode, initSync } from "bijou64/slim";
import wasmBytes from "bijou64/wasm-base64";

initSync({ module: Uint8Array.from(atob(wasmBytes), c => c.charCodeAt(0)) });
encode(42n);
```

## Errors

`decode` throws a JS `Error` with `name === "DecodeError"` and one of two
messages:

| Message                                  | Cause                                |
|------------------------------------------|--------------------------------------|
| `buffer too short for bijou64 encoding`  | Input shorter than the tag requires  |
| `bijou64 tier 8 payload overflows u64`   | Tier-8 payload + offset > `u64::MAX` |

```js
try {
  decode(new Uint8Array([0xFF, 0x00]));        // tier 8 needs 8 payload bytes
} catch (e) {
  e.name;     // "DecodeError"
  e.message;  // "buffer too short for bijou64 encoding"
}
```

`encode` and `encodedLen` throw a JS `Error` with `name === "RangeError"`
if the input `bigint` is outside `[0n, 2n ** 64n)`:

```js
try {
  encode(1n << 64n);   // exactly 2^64 — one past u64::MAX
} catch (e) {
  e.name;     // "RangeError"
  e.message;  // "bijou64: value must be in [0, 2**64)"
}

encode(-1n);           // also RangeError
encode((1n << 64n) - 1n); // OK — u64::MAX exactly
encode(0n);            // OK
```

This is a deliberate divergence from the wasm-bindgen default, which
silently truncates out-of-range `bigint` values via
`BigInt.asUintN(64, value)`. Silent truncation would let two distinct
application-level values produce the same encoded bytes, defeating
bijou's structural-canonicality guarantee at the JS boundary.

## License

Dual MIT / Apache-2.0.
