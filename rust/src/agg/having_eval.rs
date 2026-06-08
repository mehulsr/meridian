//! Port of meridian.agg.having_eval — unwired in live path.

use crate::core::types::Value;
use crate::query::ast::{AndExpr, BinaryExpr, ColumnRef, Expr, LiteralExpr};
use crate::query::operators::filter::eval_comparison;
use crate::query::result::ResultSet;

/// Evaluate a HAVING expression for a single row in `result`.
pub fn eval_having(expr: &Expr, result: &ResultSet, row_index: usize) -> Value {
    match expr {
        Expr::Column(ColumnRef { name, .. }) => {
            let col_idx = result.column_names.iter().position(|n| n == name);
            match col_idx {
                Some(i) => result.rows[row_index][i].clone(),
                None => Value::Null(crate::core::types::DataType::Bool),
            }
        }
        Expr::Literal(LiteralExpr { value }) => value.clone(),
        Expr::Binary(BinaryExpr { op, left, right }) => {
            let lv = eval_having(left, result, row_index);
            let rv = eval_having(right, result, row_index);
            eval_comparison(*op, &lv, &rv)
        }
        Expr::And(AndExpr { parts }) => {
            for part in parts {
                let val = eval_having(part, result, row_index);
                if val.is_null() || !matches!(val, Value::Bool(true)) {
                    return Value::Bool(false);
                }
            }
            Value::Bool(true)
        }
        Expr::Agg(_) => Value::Null(crate::core::types::DataType::Bool),
    }
}

/// Filter a ResultSet by a HAVING expression.
pub fn filter_having(result: &ResultSet, expr: &Expr) -> ResultSet {
    let mut kept = ResultSet::new(result.column_names.clone());
    for i in 0..result.len() {
        let val = eval_having(expr, result, i);
        if !val.is_null() && matches!(val, Value::Bool(true)) {
            kept.add_row(result.rows[i].clone());
        }
    }
    kept
}
