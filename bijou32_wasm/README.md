# bijou32_wasm

JS/Wasm bindings for [`bijou32`](../bijou32) — a canonical, length-prefixed
varint encoding for `u32` (1–5 bytes per value).

The narrower sibling of [`bijou64_wasm`](../bijou64_wasm). Unlike its
wider cousins, the wasm boundary uses plain JS `number` values, not
`bigint` — `u32::MAX` fits inside `Number.MAX_SAFE_INTEGER`, so no
`BigInt`-shimming is necessary.

## Build

```sh
bodge:32   # nix dev-shell command; output lands in dist/
```

## Usage

```js
import { encode, decode, decodeAll, encodedLen, MAX_BYTES } from "bijou32";

encode(300);                                  // Uint8Array([0xFC, 0x30])
decode(new Uint8Array([0xFC, 0x30]));         // { value: 300, bytesRead: 2 }
decodeAll(buf);                               // Uint32Array of every value in buf
encodedLen(300);                              // 2
MAX_BYTES();                                  // 5
```

`u32` crosses the boundary as plain JS `number`. `decodeAll` returns a
typed `Uint32Array` (denser than a plain array of `number`s for large
batches).

For downstream libraries shipping their own init strategy, import from
`bijou32/slim` and call `initSync({ module: ... })` yourself — see the
[wasm-bodge slim docs](https://github.com/alexjg/wasm-bodge#the-slim-escape-hatch).

## License

Dual MIT / Apache-2.0.
