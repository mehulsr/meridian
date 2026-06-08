//! Sort operator.
//!
//! Port of `meridian.query.operators.sort`. Uses `util::sort::multi_column_sort`
//! to order rows by multiple keys.

use std::collections::HashMap;

use crate::storage::column::Column;
use crate::core::types::Value;
use crate::util::sort::multi_column_sort;

/// Port of Python's `sort_rows` — sort selected rows by multiple keys.
/// `keys` is a list of `(column_name, is_descending)`.
pub fn sort_rows(
    columns: &HashMap<String, Column>,
    row_ids: &[usize],
    keys: &[(String, bool)],
) -> Vec<usize> {
    if row_ids.is_empty() {
        return Vec::new();
    }
    let key_cols: Vec<Vec<Value>> = keys
        .iter()
        .map(|(name, _)| {
            row_ids.iter().map(|&r| columns[name].get(r)).collect()
        })
        .collect();
    let descending: Vec<bool> = keys.iter().map(|(_, desc)| *desc).collect();
    let order = multi_column_sort(&key_cols, &descending);
    order.iter().map(|&i| row_ids[i]).collect()
}
