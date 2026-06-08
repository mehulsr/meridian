//! Port of `tests/test_types.py`.
//! Covers `Value` equality, `compare_values` ordering, and NULL-sorts-first.

use std::cmp::Ordering;

use meridian::core::types::{compare_values, DataType, Value};

#[test]
fn test_value_equality_and_order() {
    let a = Value::from_int(10);
    let b = Value::from_int(20);
    assert_eq!(compare_values(&a, &b), Ordering::Less);
    assert_eq!(Value::from_int(10), Value::from_int(10));
}

#[test]
fn test_null_sorts_first() {
    assert_eq!(
        compare_values(&Value::null(DataType::Int64), &Value::from_int(1)),
        Ordering::Less
    );
}
