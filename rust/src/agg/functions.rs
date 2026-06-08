//! Port of meridian.agg.functions

use crate::core::enums::AggFunc;
use crate::core::types::{DataType, Value};
use crate::storage::column::Column;

/// Apply an aggregate function over the given row ids of a column.
/// Matches Python `apply_agg` semantics exactly.
pub fn apply_agg(func: AggFunc, column: Option<&Column>, row_ids: Option<&[usize]>) -> Value {
    if func == AggFunc::Count {
        let count = match (column, row_ids) {
            (None, ids) => ids.map(|r| r.len()).unwrap_or(0),
            (Some(col), None) => col.len(),
            (Some(_), Some(ids)) => ids.len(),
        };
        return Value::Int64(count as i64);
    }

    let col = match column {
        Some(c) => c,
        None => return Value::Null(DataType::Float64),
    };

    let indices: Vec<usize> = match row_ids {
        Some(ids) => ids.to_vec(),
        None => (0..col.len()).collect(),
    };

    let values: Vec<Value> = indices.iter().map(|&r| col.get(r)).collect();
    let non_null: Vec<&Value> = values.iter().filter(|v| !v.is_null()).collect();

    match func {
        AggFunc::Sum => {
            let mut total = 0.0f64;
            for v in &non_null {
                total += match v {
                    Value::Int64(i) => *i as f64,
                    Value::Float64(f) => *f,
                    _ => 0.0,
                };
            }
            Value::Float64(total)
        }
        AggFunc::Avg => {
            if non_null.is_empty() {
                return Value::Null(DataType::Float64);
            }
            // Re-invoke SUM then divide by non-null count (matches Python two-pass)
            let sum_val = apply_agg(AggFunc::Sum, Some(col), row_ids);
            let s = match sum_val {
                Value::Float64(f) => f,
                _ => 0.0,
            };
            Value::Float64(s / non_null.len() as f64)
        }
        AggFunc::Min => {
            if non_null.is_empty() {
                return Value::Null(col.dtype);
            }
            let mut best = non_null[0].clone();
            for v in &non_null[1..] {
                if payload_lt(v, &best) {
                    best = (*v).clone();
                }
            }
            best
        }
        AggFunc::Max => {
            if non_null.is_empty() {
                return Value::Null(col.dtype);
            }
            let mut best = non_null[0].clone();
            for v in &non_null[1..] {
                if payload_gt(v, &best) {
                    best = (*v).clone();
                }
            }
            best
        }
        AggFunc::Count => unreachable!(),
    }
}

/// Python MIN/MAX use raw payload comparison (not compare_values).
pub(crate) fn payload_lt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int64(x), Value::Int64(y)) => x < y,
        (Value::Float64(x), Value::Float64(y)) => x < y,
        (Value::Str(x), Value::Str(y)) => x < y,
        (Value::Bool(x), Value::Bool(y)) => x < y,
        (Value::Timestamp(x), Value::Timestamp(y)) => x < y,
        _ => false,
    }
}

pub(crate) fn payload_gt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int64(x), Value::Int64(y)) => x > y,
        (Value::Float64(x), Value::Float64(y)) => x > y,
        (Value::Str(x), Value::Str(y)) => x > y,
        (Value::Bool(x), Value::Bool(y)) => x > y,
        (Value::Timestamp(x), Value::Timestamp(y)) => x > y,
        _ => false,
    }
}
