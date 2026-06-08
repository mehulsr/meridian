//! Append-only write-ahead log for durability experiments.
//!
//! Port of `meridian.storage.wal`. **Unwired** in the live path (ARCHITECTURE
//! §1) — ported for parity. JSON (de)serialization mirrors Python's `json`
//! module via `serde_json`; the on-disk format is one JSON object per line.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value as Json};

use crate::core::types::{DataType, Value};

/// Port of the `@dataclass WalRecord`. `payload` is the JSON object Python
/// builds: `{column: [encoded_value, ...]}`.
#[derive(Debug, Clone, PartialEq)]
pub struct WalRecord {
    pub seq: u64,
    pub table: String,
    pub payload: Json,
}

/// Port of the Python `WriteAheadLog` class.
#[derive(Debug)]
pub struct WriteAheadLog {
    pub path: PathBuf,
    pub seq: u64,
    records: Vec<WalRecord>,
}

impl WriteAheadLog {
    /// Python `WriteAheadLog(path)`.
    pub fn new(path: impl AsRef<Path>) -> WriteAheadLog {
        WriteAheadLog {
            path: path.as_ref().to_path_buf(),
            seq: 0,
            records: Vec::new(),
        }
    }

    /// Python `WriteAheadLog.append_insert` — bumps `seq`, buffers a record.
    /// Insertion-ordered columns are preserved by using the caller's order.
    pub fn append_insert(&mut self, table: &str, columns: &[(String, Vec<Value>)]) -> WalRecord {
        self.seq += 1;
        let mut payload = Map::new();
        for (name, vals) in columns {
            let arr: Vec<Json> = vals.iter().map(value_to_json).collect();
            payload.insert(name.clone(), Json::Array(arr));
        }
        let record = WalRecord {
            seq: self.seq,
            table: table.to_string(),
            payload: Json::Object(payload),
        };
        self.records.push(record.clone());
        record
    }

    /// Python `WriteAheadLog.flush` — append buffered records as JSON lines,
    /// creating parent directories, then clear the buffer.
    pub fn flush(&mut self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut fh = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        for rec in &self.records {
            let line = json!({
                "seq": rec.seq,
                "table": rec.table,
                "payload": rec.payload,
            });
            writeln!(fh, "{}", serde_json::to_string(&line)?)?;
        }
        self.records.clear();
        Ok(())
    }

    /// Python `WriteAheadLog.replay_lines` — read records back from disk.
    /// Missing file → empty (Python's `if not self.path.exists()`).
    pub fn replay_lines(&self) -> std::io::Result<Vec<WalRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&self.path)?;
        let mut out = Vec::new();
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            let obj: Json = serde_json::from_str(line)?;
            out.push(WalRecord {
                seq: obj["seq"].as_u64().unwrap_or(0),
                table: obj["table"].as_str().unwrap_or("").to_string(),
                payload: obj["payload"].clone(),
            });
        }
        Ok(out)
    }
}

/// Python `_value_to_json` — NULL → JSON null, else `{"type": NAME, "value": x}`.
fn value_to_json(v: &Value) -> Json {
    if v.is_null() {
        return Json::Null;
    }
    let (ty, val) = match v {
        Value::Int64(x) => (DataType::Int64, json!(x)),
        Value::Float64(x) => (DataType::Float64, json!(x)),
        Value::Str(s) => (DataType::String, json!(s)),
        Value::Bool(b) => (DataType::Bool, json!(b)),
        Value::Timestamp(x) => (DataType::Timestamp, json!(x)),
        Value::Null(_) => unreachable!(),
    };
    json!({ "type": ty.name(), "value": val })
}
