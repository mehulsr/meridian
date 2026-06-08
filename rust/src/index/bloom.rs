//! Bloom filter for membership tests.
//!
//! Port of `meridian.index.bloom`. Optimal bit-size and hash count via the
//! standard formulas, applied exactly as in Python. Space-efficient probabilistic
//! set member test (may-contain; false positives possible, false negatives not).

use crate::core::types::Value;
use crate::storage::column::Column;
use crate::util::hash::{fnv1a64_str, murmurish_mix};

/// Port of the Python `BloomFilter` class.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    pub size: usize,
    pub hash_count: usize,
    bits: Vec<u8>,
    count: u64,
}

impl BloomFilter {
    /// Python `BloomFilter(expected_items, false_positive_rate=0.01)`.
    pub fn new(expected_items: usize, false_positive_rate: f64) -> BloomFilter {
        let size = Self::optimal_size(expected_items, false_positive_rate);
        let hash_count = Self::optimal_hashes(size, expected_items);
        let byte_count = (size + 7) / 8;
        BloomFilter {
            size,
            hash_count,
            bits: vec![0u8; byte_count],
            count: 0,
        }
    }

    /// Python `BloomFilter._optimal_size` — `-(n * ln(p)) / (ln(2)^2)`.
    fn optimal_size(n: usize, p: f64) -> usize {
        if n == 0 {
            return 64;
        }
        let m = -(n as f64 * p.ln()) / (2.0_f64.ln() * 2.0_f64.ln());
        std::cmp::max(64, m as usize)
    }

    /// Python `BloomFilter._optimal_hashes` — `(m / n) * ln(2)`, clamped [1, 12].
    fn optimal_hashes(m: usize, n: usize) -> usize {
        if n == 0 {
            return 1;
        }
        let k = (m as f64 / n as f64) * 2.0_f64.ln();
        std::cmp::max(1, std::cmp::min(12, k as usize))
    }

    /// Python `BloomFilter._hashes` — derive `hash_count` independent indices.
    fn hashes(&self, text: &str) -> Vec<usize> {
        let h1 = fnv1a64_str(text);
        let h2 = murmurish_mix(h1);
        let mut out = Vec::with_capacity(self.hash_count);
        for i in 0..self.hash_count {
            let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
            out.push((combined % self.size as u64) as usize);
        }
        out
    }

    /// Python `BloomFilter.add`.
    pub fn add(&mut self, value: &Value) {
        if value.is_null() {
            return;
        }
        let text = if let Some(s) = value.as_str() {
            s.to_string()
        } else {
            format!("{:?}", value)
        };
        for bit_idx in self.hashes(&text) {
            let byte_idx = bit_idx >> 3;
            let bit_offset = bit_idx & 7;
            self.bits[byte_idx] |= 1u8 << bit_offset;
        }
        self.count += 1;
    }

    /// Python `BloomFilter.build` — add all non-null values from column.
    pub fn build(&mut self, column: &Column) {
        for value in column.to_list() {
            self.add(&value);
        }
    }

    /// Python `BloomFilter.maybe_contains` — check all hashes set; false positive
    /// possible if not all set (the bits are set), but no false negatives.
    pub fn maybe_contains(&self, value: &Value) -> bool {
        if value.is_null() {
            return false;
        }
        let text = if let Some(s) = value.as_str() {
            s.to_string()
        } else {
            format!("{:?}", value)
        };
        for bit_idx in self.hashes(&text) {
            let byte_idx = bit_idx >> 3;
            let bit_offset = bit_idx & 7;
            if self.bits[byte_idx] & (1u8 << bit_offset) == 0 {
                return false;
            }
        }
        true
    }

    /// Python `BloomFilter.fill_ratio` — fraction of set bits.
    pub fn fill_ratio(&self) -> f64 {
        let set_bits = self.bits.iter().map(|&b| b.count_ones() as usize).sum::<usize>();
        set_bits as f64 / std::cmp::max(1, self.size) as f64
    }
}
