//! Streaming encode and decode.
//!
//! Many real-world uses of `bijou128` involve a stream of values rather
//! than a single one — a length-prefixed framing protocol, a list of
//! offsets, a series of counters. This example shows the canonical
//! patterns for both directions.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example decode128 --package bijou128
//! ```
//!
//! Four patterns are demonstrated, in order from highest- to
//! lowest-level:
//!
//! 1. **`decode_all`** — eager batch decode into a `Vec<u128>`. The
//!    simplest spelling when you want every value or the first error.
//! 2. **`decode_iter`** — lazy `Iterator<Item = Result<u128, DecodeError>>`.
//!    Cleaner for combinator chains (`map`, `filter`, `take`).
//! 3. **`decode_iter` with `collect::<Result<Vec, _>>`** — what
//!    `decode_all` does under the hood, written out.
//! 4. **Manual slice cursor** — explicit `&buf[n..]` re-slicing.
//!    Works in any Rust environment; useful when you want to interleave
//!    other reads between bijou decodes.

use bijou128::{DecodeError, decode, decode_all, decode_iter, encode};

fn main() -> Result<(), DecodeError> {
    let values: &[u128] = &[
        0,
        1,
        42,
        239,
        240,
        495,
        496,
        65_535,
        1u128 << 64,
        1u128 << 96,
        u128::MAX,
    ];

    // -------- Encode --------
    //
    // `encode` *appends* into a Vec — call it in a loop to build up a
    // back-to-back stream. The Vec grows amortized-linearly; total work
    // is O(sum of encoded lengths).
    let mut buf = Vec::new();
    for &v in values {
        encode(v, &mut buf);
    }
    println!("encoded {} values into {} bytes", values.len(), buf.len());

    // -------- Pattern 1: decode_all (eager batch) --------
    //
    // The simplest "give me every value or fail" spelling. Allocates
    // one Vec sized to the decoded count.
    {
        let decoded = decode_all(&buf)?;
        assert_eq!(decoded, values);
        println!("[all]    decoded {} values via decode_all", decoded.len());
    }

    // -------- Pattern 2: decode_iter (lazy + combinators) --------
    //
    // `decode_iter` returns a fused `Iterator`. Use it when you want to
    // chain combinators or stop early — for example, filter on the
    // decoded values without allocating the intermediate Vec.
    {
        let count_below_1000: usize = decode_iter(&buf)
            .filter_map(Result::ok)
            .filter(|&v| v < 1000)
            .count();
        println!("[iter]   {count_below_1000} values < 1000");
    }

    // -------- Pattern 3: decode_iter::collect (what decode_all does) --------
    //
    // Equivalent to Pattern 1. Useful to know if you want a Vec but
    // also want to fuse extra iterator combinators in.
    {
        let decoded: Result<Vec<u128>, DecodeError> = decode_iter(&buf).collect();
        assert_eq!(decoded?, values);
        println!(
            "[collect] decoded {} values via decode_iter().collect::<Result<Vec, _>>()",
            values.len()
        );
    }

    // -------- Pattern 4: manual cursor --------
    //
    // The decoder returns `(value, bytes_consumed)`. The caller advances
    // its own slice cursor by `bytes_consumed`. Useful when you need to
    // interleave bijou decodes with other reads from the same buffer
    // (e.g. a length-prefixed framing protocol where each frame has a
    // bijou-encoded length followed by the framed payload).
    {
        let mut cursor: &[u8] = &buf;
        let mut decoded = Vec::new();
        while !cursor.is_empty() {
            let (value, n) = decode(cursor)?;
            decoded.push(value);
            // `decode` guarantees `n <= cursor.len()`, so this slice is
            // always in bounds. `get(n..)` keeps the example panic-free
            // even under hostile static analysis.
            cursor = cursor.get(n..).unwrap_or(&[]);
        }
        assert_eq!(decoded, values);
        println!("[manual] decoded {} values", decoded.len());
    }

    Ok(())
}
