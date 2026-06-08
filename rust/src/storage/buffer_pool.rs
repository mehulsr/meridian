//! Simple chunk decode cache — models buffer pool behavior.
//!
//! Port of `meridian.storage.buffer_pool`. **Unwired** in the live query path
//! (ARCHITECTURE §1) — ported minimally for parity. Python uses an
//! `OrderedDict` for LRU; here a `Vec` of keys tracks recency order alongside a
//! `HashMap` for O(1) lookup. Hit/miss accounting matches the Python exactly.

use std::collections::HashMap;

use crate::core::types::Value;
use crate::storage::chunk::ColumnChunk;

/// Cache key — Python's `(f"{table}:{column_name}", chunk_id)`.
type Key = (String, usize);

/// Port of the Python `DecodeCache` class.
#[derive(Debug)]
pub struct DecodeCache {
    pub capacity: usize,
    cache: HashMap<Key, Vec<Value>>,
    /// Recency order, least-recently-used first (mirrors `OrderedDict` order).
    order: Vec<Key>,
    pub hits: u64,
    pub misses: u64,
}

impl DecodeCache {
    /// Python `DecodeCache(capacity=128)`.
    pub fn new(capacity: usize) -> DecodeCache {
        DecodeCache {
            capacity,
            cache: HashMap::new(),
            order: Vec::new(),
            hits: 0,
            misses: 0,
        }
    }

    fn make_key(table: &str, chunk: &ColumnChunk, chunk_id: usize) -> Key {
        (format!("{table}:{}", chunk.column_name), chunk_id)
    }

    /// Move `key` to the most-recently-used end of `order`.
    fn touch(&mut self, key: &Key) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(pos);
            self.order.push(k);
        }
    }

    /// Python `DecodeCache.get` — increments hits/misses; on a hit moves the key
    /// to the MRU end and returns the cached values.
    pub fn get(&mut self, table: &str, chunk: &ColumnChunk, chunk_id: usize) -> Option<Vec<Value>> {
        let key = Self::make_key(table, chunk, chunk_id);
        if self.cache.contains_key(&key) {
            self.hits += 1;
            self.touch(&key);
            return self.cache.get(&key).cloned();
        }
        self.misses += 1;
        None
    }

    /// Python `DecodeCache.put` — insert/refresh, then evict LRU over capacity.
    pub fn put(&mut self, table: &str, chunk: &ColumnChunk, chunk_id: usize, values: Vec<Value>) {
        let key = Self::make_key(table, chunk, chunk_id);
        let existed = self.cache.insert(key.clone(), values).is_some();
        if existed {
            self.touch(&key);
        } else {
            self.order.push(key);
        }
        while self.cache.len() > self.capacity {
            let lru = self.order.remove(0);
            self.cache.remove(&lru);
        }
    }

    /// Python `DecodeCache.stats`.
    pub fn stats(&self) -> CacheStats {
        let total = self.hits + self.misses;
        CacheStats {
            hits: self.hits,
            misses: self.misses,
            size: self.cache.len(),
            hit_rate: if total != 0 {
                self.hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

/// Typed view of Python's `stats()` dict.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub hit_rate: f64,
}
