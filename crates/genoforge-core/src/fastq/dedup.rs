//! Bottom-k sketch for estimating the duplicate-read fraction in one pass.
//!
//! Every read sequence is hashed. The sketch keeps the `k` smallest distinct
//! hash values seen, together with how many times each occurred. Because a
//! uniform hash makes the bottom-k a uniform sample of *distinct* sequences,
//! `1 - k / (sum of their counts)` estimates the fraction of all reads that
//! repeat an earlier read. Memory is `O(k)` regardless of file size.
//!
//! Counts are exact for every hash that ends up in the sketch: the admission
//! threshold (current maximum) only ever decreases, so a hash that is in the
//! final bottom-k was below the threshold at every earlier occurrence too.
//! The same argument makes [`BottomK::merge`] exact.

use std::collections::BTreeMap;

/// Bottom-k sketch of hashed sequences with multiplicities.
#[derive(Debug, Clone)]
pub struct BottomK {
    k: usize,
    entries: BTreeMap<u64, u64>,
}

impl BottomK {
    /// Sketch keeping the `k` smallest hashes (`k = 0` is treated as `1`).
    #[must_use]
    pub fn new(k: usize) -> Self {
        Self {
            k: k.max(1),
            entries: BTreeMap::new(),
        }
    }

    /// Record one occurrence of `hash`.
    pub fn insert(&mut self, hash: u64) {
        if let Some(count) = self.entries.get_mut(&hash) {
            *count += 1;
            return;
        }
        if self.entries.len() < self.k {
            self.entries.insert(hash, 1);
            return;
        }
        if self
            .entries
            .last_key_value()
            .is_some_and(|(&max, _)| hash < max)
        {
            self.entries.insert(hash, 1);
            self.entries.pop_last();
        }
    }

    /// Union with another sketch (counts add), keeping this sketch's `k`.
    pub fn merge(&mut self, other: &Self) {
        for (&hash, &count) in &other.entries {
            *self.entries.entry(hash).or_insert(0) += count;
        }
        while self.entries.len() > self.k {
            self.entries.pop_last();
        }
    }

    /// Distinct hashes currently held (at most `k`).
    #[must_use]
    pub fn distinct(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Total occurrences of the held hashes.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.entries.values().sum()
    }

    /// Estimated fraction of reads that repeat an earlier read (`0.0` when empty).
    #[must_use]
    pub fn duplicate_fraction(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            1.0 - crate::frac(self.distinct(), total)
        }
    }
}

/// 64-bit FNV-1a. Not cryptographic; uniform enough for sketching and
/// dependency-free.
#[must_use]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes
        .iter()
        .fold(OFFSET, |h, &b| (h ^ u64::from(b)).wrapping_mul(PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_k_smallest_with_exact_counts() {
        let mut s = BottomK::new(3);
        for _ in 0..2 {
            for h in [50, 40, 30, 20, 10] {
                s.insert(h);
            }
        }
        assert_eq!(s.entries, BTreeMap::from([(10, 2), (20, 2), (30, 2)]));
        assert_eq!(s.distinct(), 3);
        assert_eq!(s.total(), 6);
        assert_eq!(s.duplicate_fraction(), 0.5);
    }

    #[test]
    fn evicted_hash_is_not_readmitted() {
        let mut s = BottomK::new(2);
        s.insert(5);
        s.insert(6);
        s.insert(1); // evicts 6
        s.insert(6); // must not come back
        assert_eq!(s.entries, BTreeMap::from([(1, 1), (5, 1)]));
    }

    #[test]
    fn merge_matches_single_stream() {
        let stream: Vec<u64> = (0..200).map(|i| (i * 7919) % 97).collect();
        let mut all = BottomK::new(10);
        let mut a = BottomK::new(10);
        let mut b = BottomK::new(10);
        for (i, &h) in stream.iter().enumerate() {
            all.insert(h);
            if i % 3 == 0 {
                a.insert(h);
            } else {
                b.insert(h);
            }
        }
        a.merge(&b);
        assert_eq!(a.entries, all.entries);
    }

    #[test]
    fn empty_sketch_reports_zero() {
        assert_eq!(BottomK::new(0).duplicate_fraction(), 0.0);
    }

    #[test]
    fn fnv1a_known_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
