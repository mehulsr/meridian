//! Min/max zone maps per chunk for predicate pushdown.
//!
//! Port of `meridian.index.zone_map`. Used to prune chunk-level scans by
//! comparing a query's low/high bounds against each chunk's (min, max) zone.

use crate::core::types::{compare_values, Value};
use crate::storage::chunk::ColumnChunk;

/// Port of the `@dataclass Zone`.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub chunk_id: usize,
    pub min_value: Value,
    pub max_value: Value,
    pub null_count: usize,
}

/// Port of the Python `ZoneMap` class.
#[derive(Debug, Clone)]
pub struct ZoneMap {
    pub column_name: String,
    pub zones: Vec<Zone>,
}

impl ZoneMap {
    /// Python `ZoneMap(column_name)`.
    pub fn new(column_name: impl Into<String>) -> ZoneMap {
        ZoneMap { column_name: column_name.into(), zones: Vec::new() }
    }

    /// Python `ZoneMap.build_from_chunks` — compute zone per chunk.
    pub fn build_from_chunks(&mut self, chunks: &[ColumnChunk]) {
        self.zones.clear();
        for (chunk_id, chunk) in chunks.iter().enumerate() {
            let values = chunk.decode_all();
            let non_null: Vec<&Value> = values.iter().filter(|v| !v.is_null()).collect();
            if non_null.is_empty() {
                continue;
            }
            let mut min_v = (*non_null[0]).clone();
            let mut max_v = min_v.clone();
            for &v in &non_null[1..] {
                if compare_values(v, &min_v) == std::cmp::Ordering::Less {
                    min_v = v.clone();
                }
                if compare_values(v, &max_v) == std::cmp::Ordering::Greater {
                    max_v = v.clone();
                }
            }
            self.zones.push(Zone {
                chunk_id,
                min_value: min_v,
                max_value: max_v,
                null_count: values.len() - non_null.len(),
            });
        }
    }

    /// Python `ZoneMap.candidate_chunks` — filter by low/high bounds.
    pub fn candidate_chunks(
        &self,
        low: Option<&Value>,
        high: Option<&Value>,
    ) -> Vec<usize> {
        let mut result = Vec::new();
        for zone in &self.zones {
            if let Some(low) = low {
                if compare_values(&zone.max_value, low) == std::cmp::Ordering::Less {
                    continue;
                }
            }
            if let Some(high) = high {
                if compare_values(&zone.min_value, high) == std::cmp::Ordering::Greater {
                    continue;
                }
            }
            result.push(zone.chunk_id);
        }
        result
    }
}
