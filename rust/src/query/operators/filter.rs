//! Filter evaluation.
//!
//! Port of `meridian.query.operators.filter`. Per-row recursive expr evaluation
//! over `Value` (parity with Python — no optimization).

use std::collections::HashMap;
use std::cmp::Ordering;

use crate::core::enums::CompareOp;
use crate::core::types::{compare_values, Value};
use crate::query::ast::{AndExpr, BinaryExpr, ColumnRef, Expr, LiteralExpr};
use crate::storage::column::Column;

/// Port of Python's `eval_expr` — recursive evaluation of an expression at a
/// specific row. Returns a Value (result of the expr evaluation).
pub fn eval_expr(
    expr: &Expr,
    columns: &HashMap<String, Column>,
    row_id: usize,
) -> Value {
    match expr {
        Expr::Column(ColumnRef { name, .. }) => columns[name].get(row_id),
        Expr::Literal(LiteralExpr { value }) => value.clone(),
        Expr::Binary(BinaryExpr { op, left, right }) => {
            let left_val = eval_expr(left, columns, row_id);
            let right_val = eval_expr(right, columns, row_id);
            eval_comparison(*op, &left_val, &right_val)
        }
        Expr::And(AndExpr { parts }) => {
            for part in parts {
                let result = eval_expr(part, columns, row_id);
                if result.is_null() || !result.as_bool().unwrap_or(false) {
                    return Value::from_bool(false);
                }
            }
            Value::from_bool(true)
        }
        Expr::Agg(_) => {
            panic!("aggregate expressions not allowed in filter context")
        }
    }
}

/// Port of Python's `eval_comparison` — apply a comparison operator to two values.
/// NULL operands → FALSE; otherwise use `compare_values` for the ordering.
pub fn eval_comparison(op: CompareOp, left: &Value, right: &Value) -> Value {
    if left.is_null() || right.is_null() {
        return Value::from_bool(false);
    }
    let cmp = compare_values(left, right);
    let result = match op {
        CompareOp::Eq => cmp == Ordering::Equal,
        CompareOp::Ne => cmp != Ordering::Equal,
        CompareOp::Lt => cmp == Ordering::Less,
        CompareOp::Le => cmp != Ordering::Greater,
        CompareOp::Gt => cmp == Ordering::Greater,
        CompareOp::Ge => cmp != Ordering::Less,
    };
    Value::from_bool(result)
}

/// Port of Python's `filter_rows` — apply a predicate to all rows, return
/// matching row IDs. Expects all columns to have the same length.
pub fn filter_rows(
    columns: &HashMap<String, Column>,
    predicate: &Expr,
) -> Vec<usize> {
    if columns.is_empty() {
        return Vec::new();
    }
    let n = columns.values().next().unwrap().len();
    let mut selected = Vec::new();
    for row_id in 0..n {
        let result = eval_expr(predicate, columns, row_id);
        if !result.is_null() && result.as_bool().unwrap_or(false) {
            selected.push(row_id);
        }
    }
    selected
}
