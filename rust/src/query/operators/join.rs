//! Nested-loop hash join (build phase uses dict of lists).
//!
//! Port of `meridian.query.operators.join`. Build a hash map on the right table,
//! then probe with the left table.

use std::collections::HashMap;

use crate::core::types::Value;
use crate::storage::column::Column;

/// Port of Python's `hash_join` — join two column dicts on key columns.
/// Returns (merged_columns, left_row_ids, right_row_ids).
pub fn hash_join(
    left: &HashMap<String, Column>,
    right: &HashMap<String, Column>,
    left_key: &str,
    right_key: &str,
) -> (HashMap<String, Column>, Vec<usize>, Vec<usize>) {
    // Build phase: collect right values (using linear scan since Value isn't Hash)
    let right_col = &right[right_key];
    let mut build: Vec<(Value, Vec<usize>)> = Vec::new();
    for i in 0..right_col.len() {
        let key = right_col.get(i);
        match build.iter_mut().find(|(v, _)| v == &key) {
            Some((_, rows)) => rows.push(i),
            None => build.push((key, vec![i])),
        }
    }

    // Probe phase: scan the left table
    let mut joined_left_ids = Vec::new();
    let mut joined_right_ids = Vec::new();
    let left_col = &left[left_key];
    for li in 0..left_col.len() {
        let key = left_col.get(li);
        if let Some((_, right_matches)) = build.iter().find(|(v, _)| v == &key) {
            for &ri in right_matches {
                joined_left_ids.push(li);
                joined_right_ids.push(ri);
            }
        }
    }

    // Materialize output columns with prefixed names
    let mut out_left = HashMap::new();
    for (name, col) in left {
        let mut new_col = Column::new(format!("l.{name}"), col.dtype);
        for &i in &joined_left_ids {
            new_col.append(col.get(i));
        }
        out_left.insert(name.clone(), new_col);
    }

    let mut out_right = HashMap::new();
    for (name, col) in right {
        let mut new_col = Column::new(format!("r.{name}"), col.dtype);
        for &i in &joined_right_ids {
            new_col.append(col.get(i));
        }
        out_right.insert(name.clone(), new_col);
    }

    let mut merged = out_left;
    for (name, col) in out_right {
        merged.insert(format!("{}__{}", right_key.split('.').last().unwrap_or(&right_key), name), col);
    }

    (merged, joined_left_ids, joined_right_ids)
}
