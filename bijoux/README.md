# bijoux

_Bijoux_, plural of [bijou] — the umbrella crate for the bijou family of
bijective, length-prefixed variable-length integer encodings.

Each bijou format crate (`bijou64`, and in future `bijou32`, `bijou128`,
the signed `bijou64s`, …) defines one canonical encoding for one integer
type, exposed as free functions. `bijoux` layers the `Encode` / `Decode`
traits directly onto those integer types, so you can write
`300u64.encode(&mut buf)` or code generic over any bijou-encodable
integer.

```rust
use bijoux::{Decode, Encode};

let mut buf = Vec::new();
300u64.encode(&mut buf);

let (value, consumed) = u64::decode(&buf).unwrap();
assert_eq!((value, consumed), (300, 2));
```

Each width lives behind a feature flag (all enabled by default) pulling
in the corresponding format crate:

| Feature | Impl for | Delegates to |
|---------|----------|--------------|
| `u64`   | `u64`    | `bijou64`    |

Depend on the individual format crates instead if you want a single
width with no facade, or the `*_wasm` crates for the npm packages.

`no_std` (uses `alloc`), `#![forbid(unsafe_code)]`.

## License

MIT OR Apache-2.0

[bijou]: https://github.com/inkandswitch/bijou
