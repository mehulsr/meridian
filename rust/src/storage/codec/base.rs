//! Column compression codecs (raw, delta, RLE, dict).
//!
//! Port of `meridian.storage.codec.base`. Four codecs dispatched through a
//! `CodecKind` enum (NOT `dyn` — see PORTING_PLAYBOOK §3.2). Binary framing
//! mirrors Python's `struct.pack`/`unpack` via `to_le_bytes`/`from_le_bytes`.
//!
//! Per-value tag bytes match Python exactly:
//!   0 = null, 1 = int64, 2 = float64, 3 = string, 4 = bool, 5 = timestamp.
//!
//! `decode_range` decodes the whole prefix `[0, end)` then slices `[start, end)`
//! — a faithful copy of the Python waste, kept for parity (optimized later).

use crate::core::types::{DataType, Value};

/// Port of the `Codec` class hierarchy + `CODECS` registry. One variant per
/// Python codec; dispatched by value rather than trait object.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CodecKind {
    Raw,
    Delta,
    Rle,
    Dict,
}

impl CodecKind {
    /// Python `Codec.name`.
    pub fn name(&self) -> &'static str {
        match self {
            CodecKind::Raw => "raw",
            CodecKind::Delta => "delta",
            CodecKind::Rle => "rle",
            CodecKind::Dict => "dict",
        }
    }

    /// Lookup mirroring Python's `CODECS[name]` dict. Returns `None` for an
    /// unknown name (callers raise the Python `ValueError`).
    pub fn from_name(name: &str) -> Option<CodecKind> {
        match name {
            "raw" => Some(CodecKind::Raw),
            "delta" => Some(CodecKind::Delta),
            "rle" => Some(CodecKind::Rle),
            "dict" => Some(CodecKind::Dict),
            _ => None,
        }
    }

    /// Python `Codec.encode`.
    pub fn encode(&self, values: &[Value]) -> Vec<u8> {
        match self {
            CodecKind::Raw => raw_encode(values),
            CodecKind::Delta => delta_encode(values),
            CodecKind::Rle => rle_encode(values),
            CodecKind::Dict => dict_encode(values),
        }
    }

    /// Python `Codec.decode_all`.
    pub fn decode_all(&self, data: &[u8], dtype: DataType, count: usize) -> Vec<Value> {
        match self {
            CodecKind::Raw => raw_decode_all(data, dtype, count),
            CodecKind::Delta => delta_decode_all(data, dtype, count),
            CodecKind::Rle => rle_decode_all(data, dtype, count),
            CodecKind::Dict => dict_decode_all(data, dtype, count),
        }
    }

    /// Python `Codec.decode_range`. Every codec decodes the prefix `[0, end)`
    /// then slices `[start, end)` (Raw decodes the whole buffer then slices).
    pub fn decode_range(
        &self,
        data: &[u8],
        dtype: DataType,
        start: usize,
        end: usize,
    ) -> Vec<Value> {
        match self {
            CodecKind::Raw => slice(raw_decode_full(data, dtype), start, end),
            _ => slice(self.decode_all(data, dtype, end), start, end),
        }
    }
}

/// Python list slice `values[start:end]`, tolerant of out-of-range bounds.
fn slice(values: Vec<Value>, start: usize, end: usize) -> Vec<Value> {
    let len = values.len();
    let s = start.min(len);
    let e = end.min(len);
    if s >= e {
        return Vec::new();
    }
    values[s..e].to_vec()
}

// ---------------------------------------------------------------------------
// RawCodec
// ---------------------------------------------------------------------------

fn raw_encode(values: &[Value]) -> Vec<u8> {
    let mut out = Vec::new();
    for v in values {
        encode_value(v, &mut out);
    }
    out
}

fn raw_decode_full(data: &[u8], dtype: DataType) -> Vec<Value> {
    decode_all_values(data, dtype)
}

fn raw_decode_all(data: &[u8], dtype: DataType, count: usize) -> Vec<Value> {
    slice(raw_decode_full(data, dtype), 0, count)
}

// ---------------------------------------------------------------------------
// DeltaCodec
// ---------------------------------------------------------------------------

fn delta_encode(values: &[Value]) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut prev = as_i64(&values[0]);
    out.extend_from_slice(&prev.to_le_bytes());
    for v in &values[1..] {
        let cur = as_i64(v);
        let delta = cur.wrapping_sub(prev);
        out.extend_from_slice(&delta.to_le_bytes());
        prev = cur;
    }
    out
}

fn delta_decode_all(data: &[u8], dtype: DataType, count: usize) -> Vec<Value> {
    if count == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(count);
    let first = read_i64(&data[0..8]);
    out.push(i64_to_value(first, dtype));
    let mut prev = first;
    let mut offset = 8;
    for _ in 0..count - 1 {
        let delta = read_i64(&data[offset..offset + 8]);
        prev = prev.wrapping_add(delta);
        out.push(i64_to_value(prev, dtype));
        offset += 8;
    }
    out
}

// ---------------------------------------------------------------------------
// RLECodec
// ---------------------------------------------------------------------------

fn rle_encode(values: &[Value]) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut run_val = &values[0];
    let mut run_len: u16 = 1;
    for v in &values[1..] {
        if v == run_val && run_len < 65535 {
            run_len += 1;
        } else {
            encode_rle_run(run_val, run_len, &mut out);
            run_val = v;
            run_len = 1;
        }
    }
    encode_rle_run(run_val, run_len, &mut out);
    out
}

fn rle_decode_all(data: &[u8], dtype: DataType, count: usize) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut offset = 0;
    while out.len() < count && offset < data.len() {
        let run_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        let val_bytes_len = data[offset] as usize;
        offset += 1;
        let val = decode_value(&data[offset..offset + val_bytes_len], dtype);
        offset += val_bytes_len;
        for _ in 0..run_len {
            out.push(val.clone());
        }
    }
    slice(out, 0, count)
}

// ---------------------------------------------------------------------------
// DictCodec
// ---------------------------------------------------------------------------

fn dict_encode(values: &[Value]) -> Vec<u8> {
    // Python keeps an `index_map: dict[Value,int]` for first-seen ordering.
    // Linear `position` lookup reproduces first-seen dedup (O(n^2); parity).
    let mut dictionary: Vec<&Value> = Vec::new();
    let mut indices: Vec<u32> = Vec::with_capacity(values.len());
    for v in values {
        let idx = match dictionary.iter().position(|d| *d == v) {
            Some(i) => i,
            None => {
                dictionary.push(v);
                dictionary.len() - 1
            }
        };
        indices.push(idx as u32);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(dictionary.len() as u32).to_le_bytes());
    for entry in &dictionary {
        let mut encoded = Vec::new();
        encode_value(entry, &mut encoded);
        out.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        out.extend_from_slice(&encoded);
    }
    for idx in &indices {
        out.extend_from_slice(&idx.to_le_bytes());
    }
    out
}

fn dict_decode_all(data: &[u8], dtype: DataType, count: usize) -> Vec<Value> {
    let dict_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut offset = 4;
    let mut dictionary: Vec<Value> = Vec::with_capacity(dict_size);
    for _ in 0..dict_size {
        let elen = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        dictionary.push(decode_value(&data[offset..offset + elen], dtype));
        offset += elen;
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let i = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        out.push(dictionary[i].clone());
        offset += 4;
    }
    out
}

// ---------------------------------------------------------------------------
// choose_codec
// ---------------------------------------------------------------------------

/// Port of `choose_codec`. Thresholds reproduced exactly (PLAYBOOK §3.3):
/// int/timestamp → delta; string with `unique < max(1, len/4)` → dict;
/// `len > 8` and `runs < len/3` → rle; else raw. Branch order matters.
pub fn choose_codec(dtype: DataType, values: &[Value]) -> CodecKind {
    if matches!(dtype, DataType::Int64 | DataType::Timestamp) && !values.is_empty() {
        return CodecKind::Delta;
    }
    if dtype == DataType::String && !values.is_empty() {
        let mut uniq: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in values {
            if !v.is_null() {
                if let Some(s) = v.as_str() {
                    uniq.insert(s);
                }
            }
        }
        if uniq.len() < std::cmp::max(1, values.len() / 4) {
            return CodecKind::Dict;
        }
    }
    if !values.is_empty() && values.len() > 8 {
        let mut runs = 1usize;
        let mut prev = &values[0];
        for v in &values[1..] {
            if v != prev {
                runs += 1;
            }
            prev = v;
        }
        if runs < values.len() / 3 {
            return CodecKind::Rle;
        }
    }
    CodecKind::Raw
}

// ---------------------------------------------------------------------------
// Per-value framing helpers
// ---------------------------------------------------------------------------

/// Python `_encode_value` — tag byte + payload, appended to `out`.
fn encode_value(v: &Value, out: &mut Vec<u8>) {
    if v.is_null() {
        out.push(0x00);
        return;
    }
    match v {
        Value::Int64(x) => {
            out.push(0x01);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Value::Float64(x) => {
            out.push(0x02);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Value::Str(s) => {
            let bytes = s.as_bytes();
            out.push(0x03);
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(bytes);
        }
        Value::Bool(b) => {
            out.push(0x04);
            out.push(if *b { 1 } else { 0 });
        }
        Value::Timestamp(x) => {
            out.push(0x05);
            out.extend_from_slice(&x.to_le_bytes());
        }
        Value::Null(_) => unreachable!(),
    }
}

/// Python `_decode_value` — read one tagged value from the front of `data`.
fn decode_value(data: &[u8], dtype: DataType) -> Value {
    match data[0] {
        0 => Value::null(dtype),
        1 => Value::from_int(read_i64(&data[1..9])),
        2 => Value::from_float(read_f64(&data[1..9])),
        3 => {
            let ln = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
            let s = std::str::from_utf8(&data[5..5 + ln])
                .expect("invalid utf-8 in string payload")
                .to_string();
            Value::from_str(s)
        }
        4 => Value::from_bool(data[1] != 0),
        5 => Value::from_timestamp(read_i64(&data[1..9])),
        tag => panic!("unknown tag {tag}"),
    }
}

/// Python `_decode_all_values` — walk the whole buffer, tag by tag.
fn decode_all_values(data: &[u8], dtype: DataType) -> Vec<Value> {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        match data[offset] {
            0 => {
                out.push(Value::null(dtype));
                offset += 1;
            }
            1 | 2 | 5 => {
                out.push(decode_value(&data[offset..offset + 9], dtype));
                offset += 9;
            }
            3 => {
                let ln = u32::from_le_bytes([
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                ]) as usize;
                out.push(decode_value(&data[offset..offset + 5 + ln], dtype));
                offset += 5 + ln;
            }
            4 => {
                out.push(decode_value(&data[offset..offset + 2], dtype));
                offset += 2;
            }
            tag => panic!("bad tag {tag} at {offset}"),
        }
    }
    out
}

/// Python `_encode_rle_run` — `<H` run length, `<B` payload length, payload.
fn encode_rle_run(value: &Value, run_len: u16, out: &mut Vec<u8>) {
    let mut encoded = Vec::new();
    encode_value(value, &mut encoded);
    out.extend_from_slice(&run_len.to_le_bytes());
    out.push(encoded.len() as u8);
    out.extend_from_slice(&encoded);
}

/// Python `_as_i64` — delta codec requires integer-like values. NULL → 0.
fn as_i64(value: &Value) -> i64 {
    if value.is_null() {
        return 0;
    }
    match value {
        Value::Int64(v) => *v,
        Value::Timestamp(v) => *v,
        _ => panic!(
            "delta codec requires integer-like values, got {:?}",
            value.data_type()
        ),
    }
}

/// Python `_i64_to_value` — timestamps stay timestamps, everything else int.
fn i64_to_value(raw: i64, dtype: DataType) -> Value {
    if dtype == DataType::Timestamp {
        Value::from_timestamp(raw)
    } else {
        Value::from_int(raw)
    }
}

fn read_i64(b: &[u8]) -> i64 {
    i64::from_le_bytes(b.try_into().expect("need 8 bytes for i64"))
}

fn read_f64(b: &[u8]) -> f64 {
    f64::from_le_bytes(b.try_into().expect("need 8 bytes for f64"))
}
