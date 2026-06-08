//! Port of tests/test_ingest.py and tests/test_validators.py

use std::collections::HashMap;
use std::io::Write;

use meridian::core::types::{DataType, Value};
use meridian::ingest::csv_reader::load_csv;
use meridian::ingest::json_reader::events_schema;
use meridian::ingest::validators::BatchValidator;

#[test]
fn test_load_csv_row_count() {
    // Write a minimal CSV to a temp file and verify row_count > 0
    let dir = std::env::temp_dir();
    let path = dir.join("test_ingest_sample.csv");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "event_id,user_id,region,category,amount,ts").unwrap();
        for i in 1..=10 {
            writeln!(f, "{},100,us-east,ads,9.99,1700000000", i).unwrap();
        }
    }
    let table = load_csv(&path, "events", None, 1000).expect("load_csv should succeed");
    assert!(table.row_count() > 0);
}

#[test]
fn test_validator_rejects_length_mismatch() {
    let schema = events_schema();
    let mut v = BatchValidator::new(schema, true);
    let mut batch: HashMap<String, Vec<Value>> = HashMap::new();
    batch.insert("event_id".into(), vec![Value::Int64(1)]);
    batch.insert("user_id".into(), vec![Value::Int64(1), Value::Int64(2)]);
    let result = v.validate_batch(&batch);
    // strict=true with mismatched lengths must return Err
    assert!(result.is_err());
}

#[test]
fn test_parse_value_types() {
    use meridian::ingest::csv_reader::parse_value;
    assert_eq!(parse_value("42", DataType::Int64), Value::Int64(42));
    assert_eq!(parse_value("3.14", DataType::Float64), Value::Float64(3.14));
    assert_eq!(parse_value("true", DataType::Bool), Value::Bool(true));
    assert_eq!(parse_value("false", DataType::Bool), Value::Bool(false));
    assert_eq!(parse_value("", DataType::Int64), Value::Null(DataType::Int64));
    assert_eq!(parse_value("null", DataType::String), Value::Null(DataType::String));
    assert_eq!(parse_value("hello", DataType::String), Value::Str("hello".into()));
}
