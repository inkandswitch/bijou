//! Census of signed-integer value distributions.
//!
//! # Why
//!
//! The signed bijou format folds the sign into the value (zigzag-style
//! bijection `i64 ↔ u64`) and reuses the unsigned tier machinery. The one
//! free design parameter is the *single-byte window*: which 248 values
//! `[-k, 247 - k]` encode in one byte. Symmetric zigzag gives `k = 124`;
//! workloads that skew positive may prefer a smaller `k`.
//!
//! This crate measures real workloads so that choice is data-driven. It is
//! internal tooling and never published.
//!
//! # Wiring into an application (e.g. hexane)
//!
//! Add a [`Census`] as a `static`, call [`Census::record`] wherever a signed
//! value would be bijou-encoded, and print the report at shutdown. Each
//! `record` is a couple of relaxed atomic increments — cheap enough to leave
//! on during profiling runs.
//!
//! ```
//! use bijou_census::Census;
//!
//! static CENSUS: Census = Census::new();
//!
//! for value in [-1, 0, 1, 300, -70_000] {
//!     CENSUS.record(value);
//! }
//!
//! let report = CENSUS.report();
//! assert_eq!(report.total, 5);
//! println!("{report}");
//! ```
//!
//! # Offline analysis
//!
//! Alternatively, dump raw values (whitespace-separated decimal `i64`s) and
//! pipe them through the bundled binary:
//!
//! ```text
//! cargo run --release -p bijou_census < values.txt
//! ```

use core::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

/// Number of single-byte codes available to a bijou-family format
/// (256 first-byte values minus the 8 length tags).
pub const WINDOW_SIZE: usize = 248;

/// Named candidate windows `[-k, 247 - k]` highlighted in the report.
///
/// `k = 0` is the unsigned baseline; `k = 124` is symmetric zigzag.
pub const CANDIDATE_KS: [i64; 5] = [0, 32, 64, 96, 124];

/// Exact per-value counters cover `[-SMALL_BOUND, SMALL_BOUND - 1]`.
///
/// Every candidate window (`0 <= k <= 248`) lies inside this range, so
/// window coverage is exact, not estimated.
const SMALL_BOUND: i64 = 256;

/// Number of exact small-value slots (`2 * SMALL_BOUND`).
const SMALL_SLOTS: usize = 512;

/// Bit-length histogram slots (`|v|` needs 0..=64 bits).
const BITS_SLOTS: usize = 65;

/// Thread-safe accumulator of signed-value statistics.
///
/// All methods take `&self`; increments use relaxed atomics.
#[derive(Debug)]
pub struct Census {
    total: AtomicU64,
    negative: AtomicU64,
    zero: AtomicU64,
    positive: AtomicU64,
    /// Exact counts for values in `[-256, 255]`; index = value + 256.
    small: [AtomicU64; SMALL_SLOTS],
    /// Bit-length histogram of `|v|` for negative values outside the small range.
    large_neg: [AtomicU64; BITS_SLOTS],
    /// Bit-length histogram of `|v|` for positive values outside the small range.
    large_pos: [AtomicU64; BITS_SLOTS],
}

impl Census {
    /// Create an empty census. Usable in `static` position.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            negative: AtomicU64::new(0),
            zero: AtomicU64::new(0),
            positive: AtomicU64::new(0),
            small: [const { AtomicU64::new(0) }; SMALL_SLOTS],
            large_neg: [const { AtomicU64::new(0) }; BITS_SLOTS],
            large_pos: [const { AtomicU64::new(0) }; BITS_SLOTS],
        }
    }

    /// Record one observed value.
    pub fn record(&self, value: i64) {
        self.total.fetch_add(1, Ordering::Relaxed);

        let sign_counter = match value.cmp(&0) {
            core::cmp::Ordering::Less => &self.negative,
            core::cmp::Ordering::Equal => &self.zero,
            core::cmp::Ordering::Greater => &self.positive,
        };
        sign_counter.fetch_add(1, Ordering::Relaxed);

        // Exact slot for small values; anything out of range falls through.
        if let Some(shifted) = value.checked_add(SMALL_BOUND)
            && let Ok(index) = usize::try_from(shifted)
            && let Some(slot) = self.small.get(index)
        {
            slot.fetch_add(1, Ordering::Relaxed);
            return;
        }

        let bits = 64 - value.unsigned_abs().leading_zeros();
        let histogram = if value < 0 {
            &self.large_neg
        } else {
            &self.large_pos
        };

        if let Ok(index) = usize::try_from(bits)
            && let Some(slot) = histogram.get(index)
        {
            slot.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Snapshot the counters into an immutable [`Report`].
    #[must_use]
    pub fn report(&self) -> Report {
        let load = |counter: &AtomicU64| counter.load(Ordering::Relaxed);

        Report {
            total: load(&self.total),
            negative: load(&self.negative),
            zero: load(&self.zero),
            positive: load(&self.positive),
            small: self.small.each_ref().map(load),
            large_neg: self.large_neg.each_ref().map(load),
            large_pos: self.large_pos.each_ref().map(load),
        }
    }
}

impl Default for Census {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of a [`Census`], with window analysis and a
/// box-drawing [`fmt::Display`] rendering.
#[derive(Clone, Copy, Debug)]
pub struct Report {
    /// Total recorded samples.
    pub total: u64,
    /// Samples `< 0`.
    pub negative: u64,
    /// Samples `== 0`.
    pub zero: u64,
    /// Samples `> 0`.
    pub positive: u64,
    /// Exact counts for values in `[-256, 255]`; index = value + 256.
    pub small: [u64; SMALL_SLOTS],
    /// Bit-length histogram of `|v|` for negative values outside `[-256, 255]`.
    pub large_neg: [u64; BITS_SLOTS],
    /// Bit-length histogram of `|v|` for positive values outside `[-256, 255]`.
    pub large_pos: [u64; BITS_SLOTS],
}

impl Report {
    /// Exact number of samples that would encode in one byte under the
    /// window `[-k, 247 - k]`. Returns 0 for `k` outside `0..=248`.
    #[must_use]
    pub fn window_coverage(&self, k: i64) -> u64 {
        if !(0..=248).contains(&k) {
            return 0;
        }

        let start = usize::try_from(SMALL_BOUND - k).unwrap_or(usize::MAX);
        self.small.iter().skip(start).take(WINDOW_SIZE).sum()
    }

    /// The `k` maximising [`Self::window_coverage`], with its coverage.
    /// Ties prefer the smaller `k` (more positive headroom).
    #[must_use]
    pub fn best_window(&self) -> (i64, u64) {
        (0..=248)
            .map(|k| (k, self.window_coverage(k)))
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
            .unwrap_or((0, 0))
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Bijou signed census — {} samples", self.total)?;
        writeln!(f)?;

        writeln!(f, "┌──────────┬──────────────┬─────────┐")?;
        writeln!(f, "│ sign     │        count │   share │")?;
        writeln!(f, "├──────────┼──────────────┼─────────┤")?;
        for (label, count) in [
            ("negative", self.negative),
            ("zero", self.zero),
            ("positive", self.positive),
        ] {
            let share = pct(count, self.total);
            writeln!(f, "│ {label:<8} │ {count:>12} │ {share:>6.2}% │")?;
        }
        writeln!(f, "└──────────┴──────────────┴─────────┘")?;
        writeln!(f)?;

        writeln!(f, "Candidate 1-byte windows, window = [-k, 247 - k]:")?;
        writeln!(f, "┌──────┬──────────────────┬──────────────┬──────────┐")?;
        writeln!(f, "│    k │ range            │  1-byte hits │ coverage │")?;
        writeln!(f, "├──────┼──────────────────┼──────────────┼──────────┤")?;
        for &k in &CANDIDATE_KS {
            self.window_row(f, k)?;
        }

        let (best_k, _) = self.best_window();
        if !CANDIDATE_KS.contains(&best_k) {
            writeln!(f, "├──────┼──────────────────┼──────────────┼──────────┤")?;
            self.window_row(f, best_k)?;
        }
        writeln!(f, "└──────┴──────────────────┴──────────────┴──────────┘")?;

        let (best_k, best_hits) = self.best_window();
        writeln!(
            f,
            "Optimal window: k = {best_k} → [{}, {}], covering {:.2}% of samples",
            -best_k,
            247 - best_k,
            pct(best_hits, self.total)
        )?;
        writeln!(f)?;

        writeln!(f, "Values outside [-256, 255], by bit length of |v|:")?;
        writeln!(f, "┌──────┬──────────────┬──────────────┐")?;
        writeln!(f, "│ bits │     negative │     positive │")?;
        writeln!(f, "├──────┼──────────────┼──────────────┤")?;
        for (bits, (neg, pos)) in self
            .large_neg
            .iter()
            .zip(self.large_pos.iter())
            .enumerate()
            .filter(|(_, (neg, pos))| **neg > 0 || **pos > 0)
        {
            writeln!(f, "│ {bits:>4} │ {neg:>12} │ {pos:>12} │")?;
        }
        writeln!(f, "└──────┴──────────────┴──────────────┘")
    }
}

impl Report {
    fn window_row(&self, f: &mut fmt::Formatter<'_>, k: i64) -> fmt::Result {
        let hits = self.window_coverage(k);
        let coverage = pct(hits, self.total);
        writeln!(
            f,
            "│ {k:>4} │ [{:>6}, {:>6}] │ {hits:>12} │ {coverage:>7.2}% │",
            -k,
            247 - k
        )
    }
}

#[expect(clippy::cast_precision_loss, reason = "display-only percentages")]
fn pct(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extremes_do_not_panic_and_bucket_as_large() {
        let census = Census::new();
        census.record(i64::MIN);
        census.record(i64::MAX);
        census.record(-257);
        census.record(256);

        let report = census.report();
        assert_eq!(report.total, 4);
        assert_eq!(report.small.iter().sum::<u64>(), 0);
        assert_eq!(report.large_neg[64], 1); // i64::MIN
        assert_eq!(report.large_pos[63], 1); // i64::MAX
        assert_eq!(report.large_neg[9], 1); // -257
        assert_eq!(report.large_pos[9], 1); // 256
    }

    #[test]
    fn small_range_boundaries_are_exact() {
        let census = Census::new();
        census.record(-256);
        census.record(255);
        census.record(0);

        let report = census.report();
        assert_eq!(report.small[0], 1); // -256
        assert_eq!(report.small[511], 1); // 255
        assert_eq!(report.small[256], 1); // 0
    }

    #[test]
    fn window_coverage_matches_window_edges() {
        let census = Census::new();
        // k = 124 window is [-124, 123]: edges in, neighbours out.
        for value in [-125, -124, 123, 124] {
            census.record(value);
        }

        let report = census.report();
        assert_eq!(report.window_coverage(124), 2); // [-124, 123]: both edges
        assert_eq!(report.window_coverage(0), 2); // [0, 247]: 123 and 124
        assert_eq!(report.window_coverage(125), 2); // [-125, 122]: -125, -124
    }

    #[test]
    fn best_window_finds_a_skew() {
        let census = Census::new();
        for _ in 0..10 {
            census.record(200); // outside symmetric zigzag window [-124, 123]
        }
        census.record(-30);

        let (k, hits) = census.report().best_window();
        assert_eq!(hits, 11);
        // Any k in 30..=47 covers both -30 and 200; ties prefer smallest.
        assert_eq!(k, 30);
    }

    #[test]
    fn sign_split_counts() {
        let census = Census::new();
        for value in [-2, -1, 0, 1, 2, 3] {
            census.record(value);
        }

        let report = census.report();
        assert_eq!(report.negative, 2);
        assert_eq!(report.zero, 1);
        assert_eq!(report.positive, 3);
    }
}
