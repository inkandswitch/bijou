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
import { encode, decode, encodedLen, MAX_BYTES } from "bijou64";

// Encode
const bytes = encode(300n);                    // Uint8Array([0xF8, 0x34])

// Decode (returns { value, bytesRead })
const { value, bytesRead } = decode(bytes);    // 300n, 2

// Predict size without encoding
encodedLen((1n << 32n) + 5n);                  // 5
MAX_BYTES();                                   // 9
```

`u64` values cross the boundary as JS `bigint`. `bytesRead` is a regular
`number`.

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

## License

Dual MIT / Apache-2.0.
