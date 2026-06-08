//! Simple bump-pointer arena for short-lived allocations during query execution.
//!
//! Port of `meridian.core.arena`. **Unwired** — the live executor path never
//! touches `Arena`/`StringInterner`, so this is a minimal faithful port (W1
//! warning: "arena/StringInterner are low priority").
//!
//! The Python `Arena.alloc` returns the value it stored; in Python that handed
//! back a reference to a heap object. In Rust we hold owned items generically
//! (`T: Clone`) and return a clone, preserving the observable "store-and-return"
//! behaviour without aliasing.

use std::collections::HashMap;

use crate::core::types::Value;

/// Port of Python `@dataclass Arena`.
///
/// Generic over the stored item type; the Python version stored `Any`. Block
/// rollover at `block_size` matches Python exactly.
#[derive(Debug, Clone)]
pub struct Arena<T> {
    blocks: Vec<Vec<T>>,
    current: Vec<T>,
    pub block_size: usize,
}

impl<T: Clone> Default for Arena<T> {
    fn default() -> Self {
        Arena { blocks: Vec::new(), current: Vec::new(), block_size: 4096 }
    }
}

impl<T: Clone> Arena<T> {
    /// Python `Arena()`.
    pub fn new() -> Self {
        Arena::default()
    }

    /// Python `Arena.reset`.
    pub fn reset(&mut self) {
        self.blocks.clear();
        self.current.clear();
    }

    /// Python `Arena.alloc` — rolls the current block when full, stores, returns
    /// (a clone of) the value.
    pub fn alloc(&mut self, value: T) -> T {
        if self.current.len() >= self.block_size {
            let full = std::mem::take(&mut self.current);
            self.blocks.push(full);
        }
        self.current.push(value.clone());
        value
    }

    /// Python `Arena.alloc_many`.
    pub fn alloc_many(&mut self, values: Vec<T>) -> Vec<T> {
        values.into_iter().map(|v| self.alloc(v)).collect()
    }

    /// Python `Arena.stats` — `(blocks, objects)`.
    pub fn stats(&self) -> (usize, usize) {
        let objects: usize =
            self.blocks.iter().map(|b| b.len()).sum::<usize>() + self.current.len();
        let blocks = self.blocks.len() + usize::from(!self.current.is_empty());
        (blocks, objects)
    }
}

/// Port of Python `StringInterner`. Returns a stable owned `String` per distinct
/// key (Python returned the first-seen `str` object; we return a clone of the
/// canonical stored string, which is observably equivalent).
#[derive(Debug, Clone, Default)]
pub struct StringInterner {
    map: HashMap<String, String>,
}

impl StringInterner {
    /// Python `StringInterner()`.
    pub fn new() -> Self {
        StringInterner::default()
    }

    /// Python `StringInterner.intern`.
    pub fn intern(&mut self, s: &str) -> String {
        if let Some(existing) = self.map.get(s) {
            return existing.clone();
        }
        self.map.insert(s.to_string(), s.to_string());
        s.to_string()
    }

    /// Python `StringInterner.size`.
    pub fn size(&self) -> usize {
        self.map.len()
    }
}

/// Convenience alias for the `Value`-holding arena the engine was designed for.
pub type ValueArena = Arena<Value>;
