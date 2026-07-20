//! Offline census analysis.
//!
//! Reads whitespace-separated decimal `i64` values from stdin (or from files
//! given as arguments) and prints the census report:
//!
//! ```text
//! cargo run --release -p bijou_census < values.txt
//! cargo run --release -p bijou_census -- run1.txt run2.txt
//! ```
//!
//! Unparseable tokens are skipped (counted on stderr) so raw logs with
//! occasional noise can be piped straight through.

use bijou_census::Census;
use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
};

fn main() -> io::Result<()> {
    let census = Census::new();
    let mut skipped: u64 = 0;

    let paths: Vec<String> = env::args().skip(1).collect();
    if paths.is_empty() {
        ingest(io::stdin().lock(), &census, &mut skipped)?;
    } else {
        for path in &paths {
            ingest(BufReader::new(File::open(path)?), &census, &mut skipped)?;
        }
    }

    if skipped > 0 {
        eprintln!("warning: skipped {skipped} unparseable token(s)");
    }

    print!("{}", census.report());
    Ok(())
}

fn ingest<R: BufRead>(reader: R, census: &Census, skipped: &mut u64) -> io::Result<()> {
    for line in reader.lines() {
        for token in line?.split_whitespace() {
            match token.parse::<i64>() {
                Ok(value) => census.record(value),
                Err(_) => *skipped += 1,
            }
        }
    }

    Ok(())
}
