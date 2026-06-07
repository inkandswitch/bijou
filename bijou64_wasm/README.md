# bijou64_wasm

JS/Wasm bindings for [`bijou64`](../bijou64) — a canonical, length-prefixed varint encoding for `u64` (1–9 bytes per value).

## Build

```sh
bodge   # nix dev-shell command; output lands in dist/
```

## Usage

```js
import { encode, decode, decodeAll, encodedLen, MAX_BYTES } from "bijou64";

encode(300n);                        // Uint8Array([0xF8, 0x34])
decode(new Uint8Array([0xF8, 0x34])); // { value: 300n, bytesRead: 2 }
decodeAll(buf);                       // BigUint64Array of every value in buf
encodedLen(300n);                     // 2
MAX_BYTES();                          // 9
```

`u64` crosses the boundary as JS `bigint`. `decodeAll` returns a typed
`BigUint64Array` (denser than a plain array of `bigint`s).

## Subpath imports

Beyond the default entry, the package exposes a few subpaths:

- `bijou64/slim` — bring-your-own init. Import this and call
  `initSync({ module: ... })` yourself; nothing auto-loads the wasm.
  See the [wasm-bodge slim docs](https://github.com/alexjg/wasm-bodge#the-slim-escape-hatch).
- `bijou64/debug` — same API, but backed by an unoptimized wasm with
  DWARF preserved, so Chrome DevTools can step through Rust source. Much
  larger; use only while debugging. A `bijou64/debug/slim` variant exists too.
- `bijou64/wasm` and `bijou64/wasm-base64` — the raw `.wasm` and a
  base64-inlined module, for custom loaders.
- `bijou64/iife` — a classic `<script>`-tag build exposing a global.

## License

Dual MIT / Apache-2.0.
