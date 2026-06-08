//! Simple in-memory B-tree for range scans.
//!
//! Port of `meridian.index.btree`. The `build` method is **O(n²)** in Python
//! (linear scan + `list.insert` + `list.index` per row) — ported faithfully
//! without optimization. This is a Phase-5 target (replace with sort-then-group).
//! Range scans use the flat `_entries` list, not the tree structure itself.

use crate::core::types::{compare_values, Value};
use crate::storage::column::Column;

/// Port of the `@dataclass LeafEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafEntry {
    pub key: Value,
    pub row_ids: Vec<usize>,
}

/// Port of the Python `BTreeIndex` class.
#[derive(Debug, Clone)]
pub struct BTreeIndex {
    pub column_name: String,
    /// Sorted list of entries by key; used for range_scan and lookup.
    entries: Vec<LeafEntry>,
}

impl BTreeIndex {
    /// Python `BTreeIndex(column_name)`.
    pub fn new(column_name: impl Into<String>) -> BTreeIndex {
        BTreeIndex { column_name: column_name.into(), entries: Vec::new() }
    }

    /// Python `BTreeIndex.build` — **O(n²) algorithm kept for parity**.
    /// For each value in the column, linearly scan `entries`, insert-scan-compare,
    /// or append. This reproduces the Python behavior exactly.
    pub fn build(&mut self, column: &Column) {
        self.entries.clear();
        for (row_id, value) in column.to_list().into_iter().enumerate() {
            if value.is_null() {
                continue;
            }
            let mut inserted = false;
            for i in 0..self.entries.len() {
                let cmp = compare_values(&self.entries[i].key, &value);
                if cmp == std::cmp::Ordering::Equal {
                    self.entries[i].row_ids.push(row_id);
                    inserted = true;
                    break;
                }
                if cmp == std::cmp::Ordering::Greater {
                    self.entries.insert(i, LeafEntry { key: value.clone(), row_ids: vec![row_id] });
                    inserted = true;
                    break;
                }
            }
            if !inserted {
                self.entries.push(LeafEntry { key: value, row_ids: vec![row_id] });
            }
        }
    }

    /// Python `BTreeIndex.range_scan` — iterate `_entries` directly (flat list).
    /// `low` ≤ key ≤ `high`. None bounds are unbounded.
    pub fn range_scan(&self, low: Option<&Value>, high: Option<&Value>) -> Vec<usize> {
        let mut result = Vec::new();
        for entry in &self.entries {
            if let Some(low) = low {
                if compare_values(&entry.key, low) == std::cmp::Ordering::Less {
                    continue;
                }
            }
            if let Some(high) = high {
                if compare_values(&entry.key, high) == std::cmp::Ordering::Greater {
                    continue;
                }
            }
            result.extend(&entry.row_ids);
        }
        result
    }

    /// Python `BTreeIndex.lookup` — exact-match from the flat `_entries`.
    pub fn lookup(&self, key: &Value) -> Vec<usize> {
        for entry in &self.entries {
            if entry.key == *key {
                return entry.row_ids.clone();
            }
        }
        Vec::new()
    }

    /// Python `BTreeIndex.size`.
    pub fn size(&self) -> usize {
        self.entries.len()
    }
}
