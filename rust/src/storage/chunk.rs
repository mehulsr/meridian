//! Compressed immutable column chunk.
//!
//! Port of `meridian.storage.chunk`. A `ColumnChunk` holds a codec-compressed
//! payload plus the metadata needed to decode it.

use crate::core::types::{DataType, Value};
use crate::storage::codec::base::{choose_codec, CodecKind};

/// Port of the `@dataclass ColumnChunk`. `codec_name` is a `CodecKind` enum
/// rather than a Python string (PORTING_PLAYBOOK §3.2).
#[derive(Debug, Clone)]
pub struct ColumnChunk {
    pub column_name: String,
    pub dtype: DataType,
    pub row_count: usize,
    pub codec_name: CodecKind,
    pub payload: Vec<u8>,
}

impl ColumnChunk {
    /// Python `ColumnChunk.get` — decode a single row.
    pub fn get(&self, index: usize) -> Value {
        self.decode_range(index, index + 1)
            .into_iter()
            .next()
            .expect("index out of range")
    }

    /// Python `ColumnChunk.decode_all`.
    pub fn decode_all(&self) -> Vec<Value> {
        self.codec_name
            .decode_all(&self.payload, self.dtype, self.row_count)
    }

    /// Python `ColumnChunk.decode_range`.
    pub fn decode_range(&self, start: usize, end: usize) -> Vec<Value> {
        self.codec_name
            .decode_range(&self.payload, self.dtype, start, end)
    }

    /// Python `ColumnChunk.encoded_size`.
    pub fn encoded_size(&self) -> usize {
        self.payload.len()
    }

    /// Python `ColumnChunk.compression_ratio`.
    pub fn compression_ratio(&self, raw_bytes: usize) -> f64 {
        if raw_bytes == 0 {
            return 1.0;
        }
        self.payload.len() as f64 / raw_bytes as f64
    }
}

/// Port of `encode_column_chunk`. `codec` is `"auto"` to pick via `choose_codec`,
/// otherwise a codec name; an unknown name panics (Python `ValueError`).
pub fn encode_column_chunk(
    name: &str,
    dtype: DataType,
    values: &[Value],
    codec: &str,
) -> ColumnChunk {
    let chosen = if codec == "auto" {
        choose_codec(dtype, values)
    } else {
        CodecKind::from_name(codec).unwrap_or_else(|| panic!("unknown codec {codec}"))
    };
    let payload = chosen.encode(values);
    ColumnChunk {
        column_name: name.to_string(),
        dtype,
        row_count: values.len(),
        codec_name: chosen,
        payload,
    }
}
