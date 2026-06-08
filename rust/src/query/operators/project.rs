//! Projection operator.
//!
//! Port of `meridian.query.operators.project`. Materializes selected columns
//! from chosen rows.

use std::collections::HashMap;

use crate::core::types::Value;
use crate::storage::column::Column;

/// Port of Python's `project_columns` — select named columns from chosen rows.
/// If names == ["*"], use all columns. Returns a dict of column-name → values.
pub fn project_columns(
    columns: &HashMap<String, Column>,
    names: &[String],
    row_ids: &[usize],
) -> HashMap<String, Vec<Value>> {
    let col_names = if names == ["*"] {
        columns.keys().cloned().collect::<Vec<_>>()
    } else {
        names.to_vec()
    };
    let mut out = HashMap::new();
    for name in col_names {
        let col = &columns[&name];
        let values = row_ids.iter().map(|&r| col.get(r)).collect();
        out.insert(name, values);
    }
    out
}
