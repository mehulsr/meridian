//! Port of meridian.ingest.json_reader — JSON-lines ingestion + events_schema.

use std::io::{BufRead, BufReader};
use std::path::Path;

use serde_json::Value as JValue;

use crate::core::errors::{MeridianError, Result};
use crate::core::schema::{ColumnDef, Schema};
use crate::core::types::{DataType, Value};
use crate::storage::table::Table;

pub fn load_jsonl(path: &Path, table_name: &str, schema: &Schema) -> Result<Table> {
    let mut table = Table::new(table_name, schema.clone());
    let file = std::fs::File::open(path).map_err(|e| MeridianError::Ingest(e.to_string()))?;
    let reader = BufReader::new(file);
    let col_names = schema.column_names();
    let batch_size = 5000usize;

    let mut batch: Vec<(String, Vec<Value>)> =
        col_names.iter().map(|n| (n.clone(), Vec::new())).collect();

    for line in reader.lines() {
        let line = line.map_err(|e| MeridianError::Ingest(e.to_string()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let obj: serde_json::Map<String, JValue> = serde_json::from_str(trimmed)
            .map_err(|e| MeridianError::Ingest(e.to_string()))?;
        for (i, col) in schema.columns.iter().enumerate() {
            let raw = obj.get(&col.name);
            batch[i].1.push(json_to_value(raw, col.dtype));
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

fn json_to_value(raw: Option<&JValue>, dtype: DataType) -> Value {
    match raw {
        None => Value::Null(dtype),
        Some(JValue::Null) => Value::Null(dtype),
        Some(v) => match dtype {
            DataType::Int64 => Value::Int64(v.as_i64().unwrap_or(0)),
            DataType::Float64 => Value::Float64(v.as_f64().unwrap_or(0.0)),
            DataType::Bool => Value::Bool(v.as_bool().unwrap_or(false)),
            DataType::Timestamp => Value::Timestamp(v.as_i64().unwrap_or(0)),
            DataType::String => Value::Str(match v {
                JValue::String(s) => s.clone(),
                _ => v.to_string(),
            }),
        },
    }
}

/// Port of Python `events_schema()`.
pub fn events_schema() -> Schema {
    let mut schema = Schema::new();
    let _ = schema.add_column(ColumnDef::with_nullable("event_id", DataType::Int64, false));
    let _ = schema.add_column(ColumnDef::new("user_id", DataType::Int64));
    let _ = schema.add_column(ColumnDef::new("region", DataType::String));
    let _ = schema.add_column(ColumnDef::new("category", DataType::String));
    let _ = schema.add_column(ColumnDef::new("amount", DataType::Float64));
    let _ = schema.add_column(ColumnDef::new("ts", DataType::Timestamp));
    schema
}
