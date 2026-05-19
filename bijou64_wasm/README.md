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

For downstream libraries shipping their own init strategy, import from
`bijou64/slim` and call `initSync({ module: ... })` yourself — see the
[wasm-bodge slim docs](https://github.com/alexjg/wasm-bodge#the-slim-escape-hatch).

## License

Dual MIT / Apache-2.0.
