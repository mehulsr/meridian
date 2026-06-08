//! Table partitioning by hash or range.
//!
//! Port of `meridian.storage.partition`. **Unwired** in the live path
//! (ARCHITECTURE §1) — minimal parity port. Routes rows to one of `buckets`
//! sub-`Table`s by hashing (or range-mod) the partition column's value.

use std::collections::HashMap;

use crate::core::errors::Result;
use crate::core::schema::Schema;
use crate::core::types::Value;
use crate::storage::table::Table;
use crate::util::hash::partition_hash;

/// Port of the `@dataclass PartitionSpec`.
#[derive(Debug, Clone)]
pub struct PartitionSpec {
    pub column: String,
    /// `"hash"` | `"range"`.
    pub strategy: String,
    pub buckets: usize,
}

impl PartitionSpec {
    /// Python defaults: `strategy="hash"`, `buckets=8`.
    pub fn new(column: impl Into<String>) -> PartitionSpec {
        PartitionSpec { column: column.into(), strategy: "hash".to_string(), buckets: 8 }
    }
}

/// Port of the `@dataclass PartitionedTable` including `__post_init__`, which
/// creates one empty `Table` per bucket when none are supplied.
#[derive(Debug, Clone)]
pub struct PartitionedTable {
    pub name: String,
    pub schema: Schema,
    pub spec: PartitionSpec,
    pub partitions: Vec<Table>,
}

impl PartitionedTable {
    /// Python `PartitionedTable(name, schema, spec)` with auto-created buckets.
    pub fn new(name: impl Into<String>, schema: Schema, spec: PartitionSpec) -> PartitionedTable {
        let name = name.into();
        let partitions = (0..spec.buckets)
            .map(|i| Table::new(format!("{name}_p{i}"), schema.clone()))
            .collect();
        PartitionedTable { name, schema, spec, partitions }
    }

    /// Python `PartitionedTable.route` → bucket index. Hash strategy uses
    /// `partition_hash`; range strategy uses Python-modulo of the int value.
    pub fn bucket_of(&self, row_values: &HashMap<String, Value>) -> Result<usize> {
        let key = &row_values[&self.spec.column];
        if self.spec.strategy == "hash" {
            Ok(partition_hash(key, self.spec.buckets as u64)? as usize)
        } else {
            // Python `int(key.as_int() or 0) % buckets` — floor modulo, always >= 0.
            let k = key.as_int().unwrap_or(0);
            Ok(k.rem_euclid(self.spec.buckets as i64) as usize)
        }
    }

    /// Python `PartitionedTable.route` — the destination partition table.
    pub fn route(&self, row_values: &HashMap<String, Value>) -> Result<&Table> {
        Ok(&self.partitions[self.bucket_of(row_values)?])
    }

    /// Python `PartitionedTable.insert_row_values` — route then single-row insert.
    pub fn insert_row_values(&mut self, row_values: &HashMap<String, Value>) -> Result<usize> {
        let bucket = self.bucket_of(row_values)?;
        let batch: Vec<(String, Vec<Value>)> = self
            .schema
            .column_names()
            .into_iter()
            .map(|name| {
                let v = row_values[&name].clone();
                (name, vec![v])
            })
            .collect();
        self.partitions[bucket].insert_column_batch(batch)
    }

    /// Python `PartitionedTable.row_count`.
    pub fn row_count(&self) -> usize {
        self.partitions.iter().map(|p| p.row_count()).sum()
    }

    /// Python `PartitionedTable.scan_all`.
    pub fn scan_all(&self) -> &[Table] {
        &self.partitions
    }
}
