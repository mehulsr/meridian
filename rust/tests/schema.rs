//! Port of `tests/test_schema.py`.
//! Covers `Schema::validate_row` coercion.

use meridian::core::schema::Schema;
use meridian::core::types::{DataType, Value};

#[test]
fn test_schema_validation() {
    let schema = Schema::from_pairs(&[
        ("a".to_string(), DataType::Int64),
        ("b".to_string(), DataType::String),
    ]);
    let row = schema
        .validate_row(&vec![Value::from_int(1), Value::from_str("x")])
        .unwrap();
    assert_eq!(row[1].as_str(), Some("x"));
}
