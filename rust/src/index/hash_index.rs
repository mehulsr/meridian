//! Hash index for equality lookups.
//!
//! Port of `meridian.index.hash_index`. Single-column index keyed by value,
//! bucketed to 16 bits (`& 0xFFFF`). Each bucket holds `(Value, [row_ids])`
//! collisions resolved by linear scan on equality.

use std::collections::HashMap;

use crate::core::types::Value;
use crate::storage::column::Column;
use crate::util::hash::hash_value;

/// Port of the Python `HashIndex` class.
#[derive(Debug, Clone)]
pub struct HashIndex {
    pub column_name: String,
    /// Bucket dict: `hash & 0xFFFF` → list of `(Value, row_ids)`.
    map: HashMap<u16, Vec<(Value, Vec<usize>)>>,
    built: bool,
}

impl HashIndex {
    /// Python `HashIndex(column_name)`.
    pub fn new(column_name: impl Into<String>) -> HashIndex {
        HashIndex {
            column_name: column_name.into(),
            map: HashMap::new(),
            built: false,
        }
    }

    /// Python `HashIndex.build` — populate from column. O(n) with per-bucket
    /// linear collision chain.
    pub fn build(&mut self, column: &Column) {
        self.map.clear();
        for (row_id, value) in column.to_list().into_iter().enumerate() {
            if value.is_null() {
                continue;
            }
            let bucket = (hash_value(&value) & 0xFFFF) as u16;
            let entries = self.map.entry(bucket).or_insert_with(Vec::new);
            let mut placed = false;
            for (existing, rows) in &mut *entries {
                if existing == &value {
                    rows.push(row_id);
                    placed = true;
                    break;
                }
            }
            if !placed {
                entries.push((value, vec![row_id]));
            }
        }
        self.built = true;
    }

    /// Python `HashIndex.lookup` — exact-match, raises if not built.
    pub fn lookup(&self, key: &Value) -> Vec<usize> {
        if !self.built {
            panic!("index not built");
        }
        let bucket = (hash_value(key) & 0xFFFF) as u16;
        if let Some(entries) = self.map.get(&bucket) {
            for (existing, rows) in entries {
                if existing == key {
                    return rows.clone();
                }
            }
        }
        Vec::new()
    }

    /// Python `HashIndex.contains`.
    pub fn contains(&self, key: &Value) -> bool {
        !self.lookup(key).is_empty()
    }

    /// Python `HashIndex.bucket_count`.
    pub fn bucket_count(&self) -> usize {
        self.map.len()
    }

    /// Python `HashIndex.entry_count` — total `(Value, rows)` pairs.
    pub fn entry_count(&self) -> usize {
        self.map.values().map(|entries| entries.len()).sum()
    }
}
