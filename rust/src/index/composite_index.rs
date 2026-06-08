//! Composite multi-column hash index.
//!
//! Port of `meridian.index.composite_index`. Extends `HashIndex` to tuple keys.

use std::collections::HashMap;

use crate::core::types::Value;
use crate::storage::column::Column;
use crate::util::hash::hash_row_key;

/// Port of the Python `CompositeHashIndex` class.
#[derive(Debug, Clone)]
pub struct CompositeHashIndex {
    pub column_names: Vec<String>,
    /// Bucket dict: `hash_row_key & 0xFFFF` → list of `(tuple, row_ids)`.
    map: HashMap<u16, Vec<(Vec<Value>, Vec<usize>)>>,
}

impl CompositeHashIndex {
    /// Python `CompositeHashIndex(column_names)`.
    pub fn new(column_names: Vec<String>) -> CompositeHashIndex {
        CompositeHashIndex { column_names, map: HashMap::new() }
    }

    /// Python `CompositeHashIndex.build` — populate from a dict of columns.
    /// Each row yields a tuple of values, nulls skip the entry.
    pub fn build(&mut self, columns: &[(String, Column)]) -> Result<(), String> {
        self.map.clear();
        let col_map: std::collections::HashMap<&str, &Column> =
            columns.iter().map(|(n, c)| (n.as_str(), c)).collect();
        let n = col_map
            .get(self.column_names[0].as_str())
            .ok_or_else(|| format!("column {} not found", self.column_names[0]))?
            .len();
        for row_id in 0..n {
            let mut key = Vec::with_capacity(self.column_names.len());
            for name in &self.column_names {
                let col = col_map.get(name.as_str()).ok_or_else(|| format!("column {name} not found"))?;
                let v = col.get(row_id);
                if v.is_null() {
                    break;
                }
                key.push(v);
            }
            if key.len() < self.column_names.len() {
                continue;
            }
            let bucket = (hash_row_key(&key) & 0xFFFF) as u16;
            let entries = self.map.entry(bucket).or_insert_with(Vec::new);
            let mut placed = false;
            for (existing, rows) in &mut *entries {
                if existing == &key {
                    rows.push(row_id);
                    placed = true;
                    break;
                }
            }
            if !placed {
                entries.push((key, vec![row_id]));
            }
        }
        Ok(())
    }

    /// Python `CompositeHashIndex.lookup` — exact-match on tuple key.
    pub fn lookup(&self, key: &[Value]) -> Vec<usize> {
        let bucket = (hash_row_key(key) & 0xFFFF) as u16;
        if let Some(entries) = self.map.get(&bucket) {
            for (existing, rows) in entries {
                if existing == key {
                    return rows.clone();
                }
            }
        }
        Vec::new()
    }
}
