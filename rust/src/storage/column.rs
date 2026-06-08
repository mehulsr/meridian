//! In-memory column storage.
//!
//! Port of `meridian.storage.column`. A `Column` is the mutable buffer that
//! accumulates `Value`s before being flushed into a compressed `ColumnChunk`.

use crate::core::types::{compare_values, DataType, Value};
use crate::storage::chunk::{encode_column_chunk, ColumnChunk};

/// Port of the Python `Column` class. `values` mirrors `_values`.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub dtype: DataType,
    values: Vec<Value>,
}

impl Column {
    /// Python `Column(name, dtype)`.
    pub fn new(name: impl Into<String>, dtype: DataType) -> Column {
        Column { name: name.into(), dtype, values: Vec::new() }
    }

    /// Python `Column.append` — type-checks non-null values against `dtype`.
    /// Python raises `TypeError`; no test depends on the raise, so a mismatch
    /// panics with the same message shape.
    pub fn append(&mut self, value: Value) {
        if !value.is_null() && value.data_type() != self.dtype {
            panic!(
                "column {} expects {:?}, got {:?}",
                self.name,
                self.dtype,
                value.data_type()
            );
        }
        self.values.push(value);
    }

    /// Python `Column.append_many`.
    pub fn append_many(&mut self, values: Vec<Value>) {
        for v in values {
            self.append(v);
        }
    }

    /// Python `Column.get`.
    pub fn get(&self, index: usize) -> Value {
        self.values[index].clone()
    }

    /// Python `Column.__len__` / `Column.len`.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Python `Column.slice` — `_values[start:end]` into a fresh column.
    pub fn slice(&self, start: usize, end: usize) -> Column {
        let len = self.values.len();
        let s = start.min(len);
        let e = end.min(len);
        let mut out = Column::new(self.name.clone(), self.dtype);
        if s < e {
            out.values = self.values[s..e].to_vec();
        }
        out
    }

    /// Python `Column.to_list`.
    pub fn to_list(&self) -> Vec<Value> {
        self.values.clone()
    }

    /// Python `Column.materialize_list`.
    pub fn materialize_list(&self) -> Vec<Value> {
        (0..self.values.len()).map(|i| self.get(i)).collect()
    }

    /// Python `Column.to_chunk`.
    pub fn to_chunk(&self, codec: &str) -> ColumnChunk {
        encode_column_chunk(&self.name, self.dtype, &self.values, codec)
    }

    /// Python `Column.extend_from_chunk`.
    pub fn extend_from_chunk(&mut self, chunk: &ColumnChunk) {
        self.values.extend(chunk.decode_all());
    }

    /// Python `Column.sum_numeric` — f64 accumulation over INT64/FLOAT64,
    /// skipping NULLs. Real `f64` arithmetic (no keying tricks here).
    pub fn sum_numeric(&self) -> f64 {
        let mut total = 0.0;
        for v in &self.values {
            if v.is_null() {
                continue;
            }
            match self.dtype {
                DataType::Int64 => total += v.as_int().unwrap_or(0) as f64,
                DataType::Float64 => total += v.as_float().unwrap_or(0.0),
                _ => {}
            }
        }
        total
    }

    /// Python `Column.min_value` — smallest non-null value (None if all null).
    /// Python compares raw payloads; `compare_values` on two non-null same-type
    /// values yields the identical ordering.
    pub fn min_value(&self) -> Option<Value> {
        self.extreme(std::cmp::Ordering::Less)
    }

    /// Python `Column.max_value`.
    pub fn max_value(&self) -> Option<Value> {
        self.extreme(std::cmp::Ordering::Greater)
    }

    fn extreme(&self, want: std::cmp::Ordering) -> Option<Value> {
        let mut best: Option<&Value> = None;
        for v in &self.values {
            if v.is_null() {
                continue;
            }
            match best {
                None => best = Some(v),
                Some(b) if compare_values(v, b) == want => best = Some(v),
                _ => {}
            }
        }
        best.cloned()
    }
}
