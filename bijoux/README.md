# bijoux

_Bijoux_ — bijective, length-prefixed variable-length integer
encodings, one canonical format per integer type.

The name is a double reading: **bijouX**, with `X` ranging over the
widths ([bijou]32, bijou64, bijou128, …), and the French plural — the
crate that holds all of the bijous.

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
| `bijoux::i32`    | bijou32s    | 5         | [specs/bijou32s.md](../specs/bijou32s.md) |
| `bijoux::i64`    | bijou64s    | 9         | [specs/bijou64s.md](../specs/bijou64s.md) |
| `bijoux::i128`   | bijou128s   | 17        | [specs/bijou128s.md](../specs/bijou128s.md) |
| `bijoux::u32`    | bijou32     | 5         | [specs/bijou32.md](../specs/bijou32.md)   |
| `bijoux::u64`    | bijou64     | 9         | [specs/bijou64.md](../specs/bijou64.md)   |
| `bijoux::u128`   | bijou128    | 17        | [specs/bijou128.md](../specs/bijou128.md) |

The signed format (`bijoux::i64`) is standard [zigzag] over the bijou64
tier scheme: 1 byte for `[-124, +123]`, canonical, but byte order is
zigzag order rather than numeric order — don't use it for memcomparable
keys.

[zigzag]: https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding

`no_std` (uses `alloc`), `#![forbid(unsafe_code)]`.

## License

Code is MIT OR Apache-2.0. The encoding specifications
([`specs/`](../specs)) are CC BY-SA 4.0.

[bijou]: https://github.com/inkandswitch/bijou
