# bijoux

_Bijoux_, plural of [bijou] — bijective, length-prefixed variable-length
integer encodings, one canonical format per integer type.

Each width is a module gated by a feature (all enabled by default), and
the `Encode` / `Decode` traits are implemented directly on the integer
types:

```rust
use bijoux::{Decode, Encode};

let mut buf = Vec::new();
300u64.encode(&mut buf);

let (value, consumed) = u64::decode(&buf).unwrap();
assert_eq!((value, consumed), (300, 2));

// Or via the per-width free functions:
bijoux::u64::encode(300, &mut buf);
```

| Module / feature | Wire format | Max bytes | Spec                       |
|------------------|-------------|-----------|----------------------------|
| `bijoux::u32`    | bijou32     | 5         | [specs/bijou32.md](./specs/bijou32.md)   |
| `bijoux::u64`    | bijou64     | 9         | [specs/bijou64.md](./specs/bijou64.md)   |
| `bijoux::u128`   | bijou128    | 17        | [specs/bijou128.md](./specs/bijou128.md) |

A signed `bijoux::i64` (wire format "bijou64s": zigzag over the bijou64
tier scheme) is planned.

> [!TIP]
> Prefer fully-qualified paths (`bijoux::u64::encode`) or the traits
> over `use bijoux::u64;` — importing a module named `u64` shadows the
> primitive type in that scope.

`no_std` (uses `alloc`), `#![forbid(unsafe_code)]`.

## License

Code is MIT OR Apache-2.0. The encoding specifications
([`specs/`](./specs)) are CC BY-SA 4.0.

[bijou]: https://github.com/inkandswitch/bijou
