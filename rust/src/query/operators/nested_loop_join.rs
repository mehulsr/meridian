//! Nested-loop join for benchmark comparison against hash join.
//!
//! Port of `meridian.query.operators.nested_loop_join`. O(n*m) but no hash
//! overhead; useful for small datasets.

use std::collections::HashMap;

use crate::core::types::Value;
use crate::storage::column::Column;

/// Port of Python's `nested_loop_join` — Cartesian product filtered by key equality.
pub fn nested_loop_join(
    left: &HashMap<String, Column>,
    right: &HashMap<String, Column>,
    left_key: &str,
    right_key: &str,
) -> HashMap<String, Column> {
    let left_col = &left[left_key];
    let right_col = &right[right_key];

    let mut out: HashMap<String, Vec<Value>> = HashMap::new();
    for name in left.keys() {
        out.insert(format!("l.{name}"), Vec::new());
    }
    for name in right.keys() {
        out.insert(format!("r.{name}"), Vec::new());
    }

    // Nested loop: for each left row, match all right rows with same key
    for li in 0..left_col.len() {
        let lval = left_col.get(li);
        for ri in 0..right_col.len() {
            if right_col.get(ri) != lval {
                continue;
            }
            // Matched: append all left values
            for (name, col) in left {
                out.get_mut(&format!("l.{name}"))
                    .unwrap()
                    .push(col.get(li));
            }
            // Append all right values
            for (name, col) in right {
                out.get_mut(&format!("r.{name}"))
                    .unwrap()
                    .push(col.get(ri));
            }
        }
    }

    // Convert lists back to columns
    let mut merged = HashMap::new();
    for (name, values) in out {
        let dtype = if values.is_empty() {
            left_col.dtype
        } else {
            values[0].data_type()
        };
        let mut col = Column::new(name.clone(), dtype);
        col.append_many(values);
        merged.insert(name, col);
    }
    merged
}
