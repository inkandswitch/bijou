# bijou128_wasm

JS/Wasm bindings for [`bijou128`](../bijou128) — a canonical, length-prefixed
varint encoding for `u128` (1–17 bytes per value).

The wider sibling of [`bijou64_wasm`](../bijou64_wasm). Same API shape (free
functions, JS `bigint` boundary), wider integer range.

## Build

```sh
bodge:128   # nix dev-shell command; output lands in dist/
```

## Usage

```js
import { encode, decode, decodeAll, encodedLen, MAX_BYTES } from "bijou128";

encode(500n);                                  // Uint8Array([0xF1, 0x00, 0x04])
decode(new Uint8Array([0xF1, 0x00, 0x04]));    // { value: 500n, bytesRead: 3 }
decodeAll(buf);                                // Array<bigint> of every value in buf
encodedLen(500n);                              // 3
MAX_BYTES();                                   // 17
```

`u128` crosses the boundary as JS `bigint`. Unlike `bijou64`'s
`decodeAll` (which returns a typed `BigUint64Array`), `bijou128.decodeAll`
returns a plain `Array<bigint>` — there is no `BigUint128Array` in the
web platform, and we don't want to silently truncate at the 64-bit mark.

## Subpath imports

Beyond the default entry, the package exposes a few subpaths:

- `bijou128/slim` — bring-your-own init. Import this and call
  `initSync({ module: ... })` yourself; nothing auto-loads the wasm.
  See the [wasm-bodge slim docs](https://github.com/alexjg/wasm-bodge#the-slim-escape-hatch).
- `bijou128/debug` — same API, but backed by an unoptimized wasm with
  DWARF preserved, so Chrome DevTools can step through Rust source. Much
  larger; use only while debugging. A `bijou128/debug/slim` variant exists too.
- `bijou128/wasm` and `bijou128/wasm-base64` — the raw `.wasm` and a
  base64-inlined module, for custom loaders.
- `bijou128/iife` — a classic `<script>`-tag build exposing a global.

## License

Dual MIT / Apache-2.0.
