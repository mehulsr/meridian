//! Row group metadata — bridges chunks for vectorized scan practice.
//!
//! Port of `meridian.storage.row_group`. A `RowGroup` owns one chunk per column
//! plus a `(min, max)` zone map per column for predicate pruning.

use std::collections::HashMap;

use crate::core::types::{compare_values, Value};
use crate::storage::chunk::ColumnChunk;

/// Port of the `@dataclass RowGroup`.
#[derive(Debug, Clone)]
pub struct RowGroup {
    pub group_id: i64,
    pub row_count: usize,
    pub chunks: HashMap<String, ColumnChunk>,
    pub zone_maps: HashMap<String, (Value, Value)>,
}

impl RowGroup {
    /// Python `RowGroup(group_id, row_count)` with empty chunk/zone maps.
    pub fn new(group_id: i64, row_count: usize) -> RowGroup {
        RowGroup {
            group_id,
            row_count,
            chunks: HashMap::new(),
            zone_maps: HashMap::new(),
        }
    }

    /// Python `RowGroup.add_chunk` — stores the chunk and computes its zone map
    /// from the non-null decoded values (min/max via payload comparison).
    pub fn add_chunk(&mut self, column: &str, chunk: ColumnChunk) {
        let values = chunk.decode_all();
        self.chunks.insert(column.to_string(), chunk);
        let mut iter = values.iter().filter(|v| !v.is_null());
        if let Some(first) = iter.next() {
            let mut min_v = first;
            let mut max_v = first;
            for v in iter {
                if compare_values(v, min_v) == std::cmp::Ordering::Less {
                    min_v = v;
                }
                if compare_values(v, max_v) == std::cmp::Ordering::Greater {
                    max_v = v;
                }
            }
            self.zone_maps
                .insert(column.to_string(), (min_v.clone(), max_v.clone()));
        }
    }

    /// Python `RowGroup.read_column` — missing column is a Python `KeyError`.
    pub fn read_column(&self, name: &str) -> Vec<Value> {
        self.chunks
            .get(name)
            .unwrap_or_else(|| panic!("{name}"))
            .decode_all()
    }

    /// Python `RowGroup.read_cell`.
    pub fn read_cell(&self, column: &str, index: usize) -> Value {
        self.chunks[column].get(index)
    }

    /// Python `RowGroup.may_match` — zone-map pruning. No zone map → always
    /// matches. A NULL bound is ignored (Python's `payload is not None` guard).
    pub fn may_match(&self, column: &str, low: Option<&Value>, high: Option<&Value>) -> bool {
        let (min_v, max_v) = match self.zone_maps.get(column) {
            Some(zm) => zm,
            None => return true,
        };
        if let Some(low) = low {
            if !low.is_null() && compare_values(max_v, low) == std::cmp::Ordering::Less {
                return false;
            }
        }
        if let Some(high) = high {
            if !high.is_null() && compare_values(min_v, high) == std::cmp::Ordering::Greater {
                return false;
            }
        }
        true
    }
}

/// Port of the `@dataclass RowGroupCollection`.
#[derive(Debug, Clone, Default)]
pub struct RowGroupCollection {
    pub groups: Vec<RowGroup>,
}

impl RowGroupCollection {
    /// Python `RowGroupCollection(groups=[...])`.
    pub fn new(groups: Vec<RowGroup>) -> RowGroupCollection {
        RowGroupCollection { groups }
    }

    /// Python `RowGroupCollection.append`.
    pub fn append(&mut self, group: RowGroup) {
        self.groups.push(group);
    }

    /// Python `RowGroupCollection.total_rows`.
    pub fn total_rows(&self) -> usize {
        self.groups.iter().map(|g| g.row_count).sum()
    }

    /// Python `RowGroupCollection.scan` — concatenate matching groups' columns,
    /// skipping groups pruned by the zone map.
    pub fn scan(
        &self,
        column: &str,
        predicate_low: Option<&Value>,
        predicate_high: Option<&Value>,
    ) -> Vec<Value> {
        let mut out = Vec::new();
        for group in &self.groups {
            if !group.may_match(column, predicate_low, predicate_high) {
                continue;
            }
            out.extend(group.read_column(column));
        }
        out
    }
}
