//! Table storage with chunked columns.
//!
//! Port of `meridian.storage.table`. Pulled forward from W4 because
//! `core::catalog` (W3) hard-depends on `Table` — in Python that edge is hidden
//! behind `TYPE_CHECKING`, but in Rust it is a normal (still acyclic) dependency.

use std::collections::HashMap;

use crate::core::errors::Result;
use crate::core::schema::Schema;
use crate::core::types::{Row, Value};
use crate::index::bloom::BloomFilter;
use crate::index::hash_index::HashIndex;
use crate::storage::chunk::ColumnChunk;
use crate::storage::column::Column;

/// Python `DEFAULT_CHUNK_SIZE`.
pub const DEFAULT_CHUNK_SIZE: usize = 8192;

/// Heterogeneous index slot. Python stored arbitrary objects in `_indexes`;
/// ARCHITECTURE/PLAYBOOK call for an enum. Variants are added in W5 as the
/// index types are ported; for now it carries only what later waves attach.
#[derive(Debug, Clone)]
pub enum IndexKind {
    Unbuilt,
    Hash(HashIndex),
    Bloom(BloomFilter),
}

/// Port of the `@dataclass Table`.
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub schema: Schema,
    pub chunk_size: usize,
    columns: HashMap<String, Column>,
    chunks: HashMap<String, Vec<ColumnChunk>>,
    row_count: usize,
    indexes: HashMap<String, IndexKind>,
}

impl Table {
    /// Python `Table(name, schema)` including `__post_init__` column setup.
    pub fn new(name: impl Into<String>, schema: Schema) -> Table {
        Table::with_chunk_size(name, schema, DEFAULT_CHUNK_SIZE)
    }

    /// Python `Table(name, schema, chunk_size=...)`.
    pub fn with_chunk_size(name: impl Into<String>, schema: Schema, chunk_size: usize) -> Table {
        let mut columns = HashMap::new();
        let mut chunks = HashMap::new();
        for col_def in &schema.columns {
            columns.insert(col_def.name.clone(), Column::new(col_def.name.clone(), col_def.dtype));
            chunks.insert(col_def.name.clone(), Vec::new());
        }
        Table {
            name: name.into(),
            schema,
            chunk_size,
            columns,
            chunks,
            row_count: 0,
            indexes: HashMap::new(),
        }
    }

    /// Python `Table.row_count`.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Python `Table.insert_rows` — validate each row, then column-batch insert.
    pub fn insert_rows(&mut self, rows: &[Row]) -> Result<usize> {
        let col_names = self.schema.column_names();
        let mut batch: Vec<(String, Vec<Value>)> =
            col_names.iter().map(|n| (n.clone(), Vec::new())).collect();
        for row in rows {
            let validated = self.schema.validate_row(row)?;
            for (i, cell) in validated.into_iter().enumerate() {
                batch[i].1.push(cell);
            }
        }
        self.insert_column_batch(batch)
    }

    /// Python `Table.insert_column_batch` — equal-length columns, append+flush.
    pub fn insert_column_batch(&mut self, columns: Vec<(String, Vec<Value>)>) -> Result<usize> {
        let lengths: std::collections::HashSet<usize> =
            columns.iter().map(|(_, v)| v.len()).collect();
        if lengths.len() != 1 {
            return Err(crate::core::errors::MeridianError::Schema(format!(
                "column length mismatch: {lengths:?}"
            )));
        }
        let n = lengths.into_iter().next().unwrap_or(0);
        if n == 0 {
            return Ok(0);
        }
        for (name, values) in columns {
            let col = self.columns.get_mut(&name).expect("unknown column");
            col.append_many(values);
            self.flush_if_needed(&name);
        }
        self.row_count += n;
        Ok(n)
    }

    /// Python `Table._flush_if_needed` — flush full chunks, keep the remainder.
    fn flush_if_needed(&mut self, name: &str) {
        while self.columns[name].len() >= self.chunk_size {
            let col = &self.columns[name];
            let to_flush = col.slice(0, self.chunk_size);
            let remainder = col.slice(self.chunk_size, col.len());
            let chunk = to_flush.to_chunk("auto");
            self.chunks.get_mut(name).unwrap().push(chunk);
            self.columns.insert(name.to_string(), remainder);
        }
    }

    /// Python `Table.finalize_chunks` — flush any trailing buffered values.
    pub fn finalize_chunks(&mut self) {
        let names: Vec<String> = self.columns.keys().cloned().collect();
        for name in names {
            let col = &self.columns[&name];
            if col.len() > 0 {
                let chunk = col.to_chunk("auto");
                let dtype = col.dtype;
                self.chunks.get_mut(&name).unwrap().push(chunk);
                self.columns.insert(name.clone(), Column::new(name.clone(), dtype));
            }
        }
    }

    /// Python `Table.scan_column` — rebuild a merged `Column` from all chunks
    /// plus the still-buffered tail. (Re-decodes every chunk every call; this
    /// is the hot-path waste called out in ARCHITECTURE §3, optimized later.)
    pub fn scan_column(&self, name: &str) -> Column {
        let base = self.columns.get(name).unwrap_or_else(|| panic!("{name}"));
        let mut merged = Column::new(name, base.dtype);
        for chunk in &self.chunks[name] {
            merged.extend_from_chunk(chunk);
        }
        merged.append_many(base.to_list());
        merged
    }

    /// Python `Table.get_chunk` — out-of-range is a Python `IndexError`.
    pub fn get_chunk(&self, column: &str, chunk_id: usize) -> &ColumnChunk {
        let chunks = &self.chunks[column];
        if chunk_id >= chunks.len() {
            panic!("{chunk_id}");
        }
        &chunks[chunk_id]
    }

    /// Python `Table.chunk_count` — for one column, or the first schema column.
    pub fn chunk_count(&self, column: Option<&str>) -> usize {
        match column {
            Some(c) => self.chunks[c].len(),
            None => {
                let first = &self.schema.column_names()[0];
                self.chunks[first].len()
            }
        }
    }

    /// Python `Table.attach_index`.
    pub fn attach_index(&mut self, name: &str, index: IndexKind) {
        self.indexes.insert(name.to_string(), index);
    }

    /// Python `Table.get_index`.
    pub fn get_index(&self, name: &str) -> Option<&IndexKind> {
        self.indexes.get(name)
    }

    /// Python `Table.read_row` — materialize one row across all columns.
    pub fn read_row(&self, row_id: usize) -> Row {
        if row_id >= self.row_count {
            panic!("{row_id}");
        }
        self.schema
            .column_names()
            .iter()
            .map(|name| self.scan_column(name).get(row_id))
            .collect()
    }

    /// Python `Table.stats` — encoded vs raw-estimate byte sizes plus counts.
    pub fn stats(&self) -> TableStats {
        let raw_estimate = self.row_count * self.schema.columns.len() * 16;
        let mut encoded = 0;
        for chunks in self.chunks.values() {
            for ch in chunks {
                encoded += ch.encoded_size();
            }
        }
        let mut indexes: Vec<String> = self.indexes.keys().cloned().collect();
        indexes.sort();
        TableStats {
            rows: self.row_count,
            chunks: self.chunk_count(None),
            encoded_bytes: encoded,
            raw_estimate,
            indexes,
        }
    }
}

/// Typed view of Python `Table.stats()`'s dict.
#[derive(Debug, Clone, PartialEq)]
pub struct TableStats {
    pub rows: usize,
    pub chunks: usize,
    pub encoded_bytes: usize,
    pub raw_estimate: usize,
    pub indexes: Vec<String>,
}
