//! Port of `tests/test_codecs.py` (W2: storage::codec::base + storage::chunk).
//!
//! Plus per-codec self-consistency (roundtrip) tests written before moving on,
//! as required by the W2 wave warning.

use meridian::core::types::{DataType, Value};
use meridian::storage::chunk::encode_column_chunk;
use meridian::storage::codec::base::CodecKind;

// --- ported from test_codecs.py ---------------------------------------------

#[test]
fn test_delta_codec_roundtrip() {
    let values: Vec<Value> = (0..100).map(Value::from_int).collect();
    let chunk = encode_column_chunk("x", DataType::Int64, &values, "delta");
    let decoded = chunk.decode_all();
    assert_eq!(decoded, values);
}

#[test]
fn test_dict_codec_on_repeated_strings() {
    let mut values: Vec<Value> = vec![Value::from_str("us-east"); 50];
    values.extend(vec![Value::from_str("eu-central"); 50]);
    let chunk = encode_column_chunk("region", DataType::String, &values, "dict");
    assert_eq!(chunk.decode_range(10, 20), values[10..20].to_vec());
}

#[test]
fn test_codec_slice_still_decodes_prefix() {
    let codec = CodecKind::Delta;
    let values: Vec<Value> = (0..200).map(Value::from_int).collect();
    let payload = codec.encode(&values);
    let full = codec.decode_range(&payload, DataType::Int64, 100, 120);
    assert_eq!(full.len(), 20);
}

// --- per-codec roundtrip (self-consistency) ---------------------------------

#[test]
fn test_raw_roundtrip_mixed() {
    let values = vec![
        Value::from_int(7),
        Value::null(DataType::Int64),
        Value::from_float(3.5),
        Value::from_str("hello"),
        Value::from_bool(true),
        Value::from_bool(false),
        Value::from_timestamp(1700000000),
    ];
    let codec = CodecKind::Raw;
    let payload = codec.encode(&values);
    assert_eq!(codec.decode_all(&payload, DataType::Int64, values.len()), values);
}

#[test]
fn test_delta_roundtrip_timestamps() {
    let values: Vec<Value> = (0..50).map(|i| Value::from_timestamp(1_000 + i * 13)).collect();
    let codec = CodecKind::Delta;
    let payload = codec.encode(&values);
    assert_eq!(codec.decode_all(&payload, DataType::Timestamp, values.len()), values);
}

#[test]
fn test_rle_roundtrip_runs() {
    let mut values: Vec<Value> = vec![Value::from_int(1); 30];
    values.extend(vec![Value::from_int(2); 10]);
    values.extend(vec![Value::from_int(1); 5]);
    let codec = CodecKind::Rle;
    let payload = codec.encode(&values);
    assert_eq!(codec.decode_all(&payload, DataType::Int64, values.len()), values);
}

#[test]
fn test_dict_roundtrip_full() {
    let mut values: Vec<Value> = vec![Value::from_str("a"); 20];
    values.extend(vec![Value::from_str("b"); 20]);
    values.extend(vec![Value::from_str("a"); 10]);
    let codec = CodecKind::Dict;
    let payload = codec.encode(&values);
    assert_eq!(codec.decode_all(&payload, DataType::String, values.len()), values);
}

#[test]
fn test_chunk_get_single_row() {
    let values: Vec<Value> = (0..20).map(Value::from_int).collect();
    let chunk = encode_column_chunk("x", DataType::Int64, &values, "auto");
    assert_eq!(chunk.get(7), Value::from_int(7));
    assert_eq!(chunk.row_count, 20);
}
