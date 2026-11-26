//! Test-only helpers shared by the width modules.

use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

/// A `Hasher` that records every byte written to it, so tests can
/// compare exactly what two values feed into a hash.
#[derive(Debug, Default)]
struct Recorder(Vec<u8>);

impl Hasher for Recorder {
    fn finish(&self) -> u64 {
        0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

/// The exact byte stream `value` feeds to a `Hasher`.
pub(crate) fn hash_stream<T: Hash + ?Sized>(value: &T) -> Vec<u8> {
    let mut recorder = Recorder::default();
    value.hash(&mut recorder);
    recorder.0
}
