//! Port of meridian.ingest.schema_inference
//!
//! Threshold: ≤100 samples per column (matches Python SchemaInference._infer_column).

use crate::core::errors::{MeridianError, Result};
use crate::core::schema::{ColumnDef, Schema};
use crate::core::types::DataType;

pub struct SchemaInference;

impl SchemaInference {
    pub fn new() -> Self {
        SchemaInference
    }

    pub fn infer_from_rows(&self, header: &[String], rows: &[Vec<String>]) -> Schema {
        let mut col_samples: Vec<Vec<String>> = vec![Vec::new(); header.len()];
        for row in rows {
            for (i, _name) in header.iter().enumerate() {
                if i < row.len() {
                    col_samples[i].push(row[i].clone());
                }
            }
        }
        let mut schema = Schema::new();
        for (i, name) in header.iter().enumerate() {
            let dtype = self.infer_column(&col_samples[i]);
            let nullable =
                col_samples[i].iter().any(|s| s.is_empty() || s.to_lowercase() == "null");
            let _ = schema.add_column(ColumnDef::with_nullable(name, dtype, nullable));
        }
        schema
    }

    /// Inspects up to 100 samples. Returns the most-specific type that fits all non-null values.
    fn infer_column(&self, samples: &[String]) -> DataType {
        let mut int_count = 0usize;
        let mut float_count = 0usize;
        let mut bool_count = 0usize;
        let mut checked = 0usize;
        for s in samples.iter().take(100) {
            let sl = s.to_lowercase();
            if s.is_empty() || sl == "null" {
                continue;
            }
            checked += 1;
            if sl == "true" || sl == "false" || s == "0" || s == "1" {
                bool_count += 1;
            }
            if s.parse::<i64>().is_ok() {
                int_count += 1;
                continue;
            }
            if s.parse::<f64>().is_ok() {
                float_count += 1;
            }
        }
        if checked == 0 {
            return DataType::String;
        }
        if bool_count == checked {
            return DataType::Bool;
        }
        if float_count > 0 {
            return DataType::Float64;
        }
        if int_count == checked {
            return DataType::Int64;
        }
        DataType::String
    }

    pub fn merge_schemas(&self, a: &Schema, b: &Schema) -> Result<Schema> {
        let a_names: Vec<_> = a.columns.iter().map(|c| &c.name).collect();
        let b_names: Vec<_> = b.columns.iter().map(|c| &c.name).collect();
        if a_names != b_names {
            return Err(MeridianError::Schema("schema column names differ".into()));
        }
        let mut out = Schema::new();
        for (ca, cb) in a.columns.iter().zip(b.columns.iter()) {
            let dtype = if ca.dtype == cb.dtype { ca.dtype } else { DataType::String };
            let _ =
                out.add_column(ColumnDef::with_nullable(&ca.name, dtype, ca.nullable || cb.nullable));
        }
        Ok(out)
    }
}

impl Default for SchemaInference {
    fn default() -> Self {
        SchemaInference::new()
    }
}
