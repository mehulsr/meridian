//! Dense and sparse bitmap utilities for filters and indexes.
//!
//! Port of `meridian.util.bitmap`. Bit twiddling maps 1:1 to Python.
//!
//! Faithfulness note: Python `set(index)` past `size` calls `_resize`, which
//! reallocates `_words` to all-zero — **destroying previously-set bits**. That
//! is a latent quirk in the source, but a faithful port must reproduce it, so
//! `set` here resets the word vector on grow exactly as Python does. Callers in
//! the live path size the bitset up front and never grow it, so this is inert.

/// Port of Python `Bitset`.
#[derive(Debug, Clone, Default)]
pub struct Bitset {
    words: Vec<u64>,
    pub size: usize,
}

impl Bitset {
    /// Python `Bitset(size=0)`.
    pub fn new(size: usize) -> Self {
        let mut b = Bitset { words: Vec::new(), size: 0 };
        if size != 0 {
            b.resize(size);
        }
        b
    }

    /// Python `Bitset._resize` — sets `size` and zeroes a fresh word vector.
    fn resize(&mut self, size: usize) {
        self.size = size;
        let word_count = size.div_ceil(64);
        self.words = vec![0u64; word_count];
    }

    /// Python `Bitset.set`. Growing reallocates-to-zero (see module note).
    pub fn set(&mut self, index: usize) {
        if index >= self.size {
            self.resize(index + 1);
        }
        let word = index >> 6;
        let bit = index & 63;
        self.words[word] |= 1u64 << bit;
    }

    /// Python `Bitset.clear`. Out-of-range index panics (Python `IndexError`).
    pub fn clear(&mut self, index: usize) {
        let word = index >> 6;
        let bit = index & 63;
        self.words[word] &= !(1u64 << bit);
    }

    /// Python `Bitset.get`.
    pub fn get(&self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }
        let word = index >> 6;
        let bit = index & 63;
        (self.words[word] & (1u64 << bit)) != 0
    }

    /// Python `Bitset.and_with` — new bitset of `max(size)`, word-wise AND.
    pub fn and_with(&self, other: &Bitset) -> Bitset {
        let n = self.size.max(other.size);
        let mut out = Bitset::new(n);
        let max_words = self.words.len().max(other.words.len());
        for i in 0..max_words {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            if i < out.words.len() {
                out.words[i] = a & b;
            }
        }
        out
    }

    /// Python `Bitset.or_with` — new bitset of `max(size)`, word-wise OR.
    pub fn or_with(&self, other: &Bitset) -> Bitset {
        let n = self.size.max(other.size);
        let mut out = Bitset::new(n);
        let max_words = self.words.len().max(other.words.len());
        while out.words.len() < max_words {
            out.words.push(0);
        }
        for i in 0..max_words {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = other.words.get(i).copied().unwrap_or(0);
            out.words[i] = a | b;
        }
        out
    }

    /// Python `Bitset.iter_set_bits` — ascending list of set bit positions.
    pub fn iter_set_bits(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for (word_idx, &word) in self.words.iter().enumerate() {
            if word == 0 {
                continue;
            }
            let base = word_idx << 6;
            let mut w = word;
            let mut bit = 0usize;
            while w != 0 {
                if w & 1 != 0 {
                    result.push(base + bit);
                }
                w >>= 1;
                bit += 1;
            }
        }
        result
    }

    /// Python `Bitset.popcount`.
    pub fn popcount(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }
}
