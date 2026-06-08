//! Port of meridian.ingest.csv_reader
//!
//! Schema inference uses ≤20 samples per column (matches Python _infer_column_type).
//! Reads first 100 rows for inference, then re-reads for ingestion, batching at 5000.

use std::path::Path;

use crate::core::errors::{MeridianError, Result};
use crate::core::schema::{ColumnDef, Schema};
use crate::core::types::{DataType, Value};
use crate::storage::table::Table;

pub fn infer_schema_from_header(header: &[String], sample_rows: &[Vec<String>]) -> Schema {
    let mut schema = Schema::new();
    for (col_idx, name) in header.iter().enumerate() {
        let samples: Vec<String> =
            sample_rows.iter().filter_map(|row| row.get(col_idx).cloned()).collect();
        let dtype = infer_column_type(&samples);
        let _ = schema.add_column(ColumnDef::new(name, dtype));
    }
    schema
}

/// Inspects up to 20 samples, returns the first matching type (matches Python first-match logic).
fn infer_column_type(samples: &[String]) -> DataType {
    for s in samples.iter().take(20) {
        let sl = s.to_lowercase();
        if s.is_empty() || sl == "null" {
            continue;
        }
        if sl == "true" || sl == "false" {
            return DataType::Bool;
        }
        if s.parse::<i64>().is_ok() {
            return DataType::Int64;
        }
        if s.parse::<f64>().is_ok() {
            return DataType::Float64;
        }
    }
    DataType::String
}

pub fn parse_value(text: &str, dtype: DataType) -> Value {
    let tl = text.to_lowercase();
    if text.is_empty() || tl == "null" {
        return Value::Null(dtype);
    }
    match dtype {
        DataType::Int64 => text.parse::<i64>().map(Value::Int64).unwrap_or(Value::Null(dtype)),
        DataType::Float64 => text.parse::<f64>().map(Value::Float64).unwrap_or(Value::Null(dtype)),
        DataType::Bool => Value::Bool(tl == "1" || tl == "true" || tl == "yes"),
        DataType::Timestamp => {
            text.parse::<i64>().map(Value::Timestamp).unwrap_or(Value::Null(dtype))
        }
        DataType::String => Value::Str(text.to_string()),
    }
}

pub fn load_csv(
    path: &Path,
    table_name: &str,
    schema: Option<Schema>,
    batch_size: usize,
) -> Result<Table> {
    // First pass: collect up to 100 rows for inference
    let inferred_schema = if schema.is_none() {
        let mut rdr = csv::Reader::from_path(path)
            .map_err(|e| MeridianError::Ingest(e.to_string()))?;
        let header: Vec<String> =
            rdr.headers().map_err(|e| MeridianError::Ingest(e.to_string()))?.iter().map(String::from).collect();
        let mut sample_rows: Vec<Vec<String>> = Vec::new();
        for result in rdr.records().take(100) {
            let record = result.map_err(|e| MeridianError::Ingest(e.to_string()))?;
            sample_rows.push(record.iter().map(String::from).collect());
        }
        Some(infer_schema_from_header(&header, &sample_rows))
    } else {
        None
    };
    let schema = schema.or(inferred_schema).unwrap();

    // Second pass: ingest all rows
    let mut table = Table::new(table_name, schema.clone());
    let mut rdr = csv::Reader::from_path(path)
        .map_err(|e| MeridianError::Ingest(e.to_string()))?;

    let col_names = schema.column_names();
    let mut batch: Vec<(String, Vec<Value>)> =
        col_names.iter().map(|n| (n.clone(), Vec::new())).collect();

    for result in rdr.records() {
        let record = result.map_err(|e| MeridianError::Ingest(e.to_string()))?;
        for (i, col_def) in schema.columns.iter().enumerate() {
            let text = record.get(i).unwrap_or("");
            batch[i].1.push(parse_value(text, col_def.dtype));
        }
        if batch[0].1.len() >= batch_size {
            table.insert_column_batch(batch.clone())?;
            for (_, vals) in &mut batch {
                vals.clear();
            }
        }
    }
    if !batch[0].1.is_empty() {
        table.insert_column_batch(batch)?;
    }
    table.finalize_chunks();
    Ok(table)
}
