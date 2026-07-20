# @inkandswitch/bijoux (bijoux_wasm)

Wasm/JavaScript bindings for the [bijoux] family — bijective,
length-prefixed varint encodings for `u32`, `u64`, and `u128` in one npm
package.

Flat, width-suffixed exports (tree-shakeable free functions):

```js
import {
  decodeU64,
  decodeAllU64,
  encodeU64,
  encodedLenU64,
  MAX_BYTES_U64,
} from "@inkandswitch/bijoux";

encodeU64(300n);            // Uint8Array([0xF8, 0x34])
decodeU64(encodeU64(300n)); // Decoded64 { value: 300n, bytesRead: 2 }
```

| Family | Carrier type | `decodeAll*` returns  | Max bytes |
|--------|--------------|-----------------------|-----------|
| `U32`  | `number`     | `Uint32Array`         | 5         |
| `U64`  | `bigint`     | `BigUint64Array`      | 9         |
| `U128` | `bigint`     | `Array<bigint>`       | 17        |

Each width exposes `encode*`, `decode*`, `decodeAll*`, `encodedLen*`,
`MAX_BYTES_*()`, and a `Decoded{32,64,128}` result class. Decode errors
throw `Error` with `name === "Bijou{32,64,128}DecodeError"`; wrong-type
and out-of-range inputs throw native `TypeError` / `RangeError`.

Built into a universal npm package via [wasm-bodge] (`bodge` in the dev
shell). Tests: `wasm:test:node` (Rust ↔ wasm-bindgen ABI),
`test:js:node` (Mocha against `dist/esm/node.js`), `test:js:browser`
(Playwright against `dist/esm/web.js`).

## License

MIT OR Apache-2.0

[bijoux]: https://github.com/inkandswitch/bijou
[wasm-bodge]: https://github.com/alexjg/wasm-bodge
