//! Sorting utilities — multiple code paths for benchmarking.
//!
//! Port of `meridian.util.sort`. All routines return permutations of row
//! indices and use stable sorts (Python's `list.sort` is stable).
//!
//! Two Python quirks reproduced in `multi_column_sort`:
//!   1. The null/non-null discriminator (`0` vs `1` in the sort tuple) is never
//!      negated, so NULLs sort first regardless of `descending`.
//!   2. `_negate_payload` only flips values implementing `__neg__`; Python `str`
//!      has none, so **descending on a STRING column is a no-op** (stays
//!      ascending). Numeric/bool/timestamp columns negate, reversing their order.

use crate::core::types::{compare_values, Value};

/// Python `insertion_sort_indices` — stable insertion sort of indices via
/// `compare_values` (`> 0` ⇒ `Ordering::Greater`).
pub fn insertion_sort_indices(values: &[Value]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..values.len()).collect();
    for i in 1..indices.len() {
        let mut j = i;
        while j > 0
            && compare_values(&values[indices[j - 1]], &values[indices[j]])
                == std::cmp::Ordering::Greater
        {
            indices.swap(j - 1, j);
            j -= 1;
        }
    }
    indices
}

/// Python `timsort_values` — sort indices by `(type.name, payload)`.
///
/// Within a homogeneously-typed column the type name is constant, so the order
/// reduces to payload order with NULL first — exactly what `compare_values`
/// provides. Across mixed types we break ties by `DataType::name()` (Python's
/// `type.name` string), preserving the lexicographic primary key.
pub fn timsort_values(values: &[Value]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..values.len()).collect();
    indices.sort_by(|&a, &b| {
        let (va, vb) = (&values[a], &values[b]);
        va.data_type()
            .name()
            .cmp(vb.data_type().name())
            .then_with(|| compare_values(va, vb))
    });
    indices
}

/// Python `multi_column_sort` — lexicographic multi-key sort with per-column
/// `descending`. See module docs for the two faithfulness quirks.
pub fn multi_column_sort(keys: &[Vec<Value>], descending: &[bool]) -> Vec<usize> {
    let n = if keys.is_empty() { 0 } else { keys[0].len() };
    let mut rows: Vec<usize> = (0..n).collect();
    rows.sort_by(|&ra, &rb| {
        for (col_idx, col) in keys.iter().enumerate() {
            let (a, b) = (&col[ra], &col[rb]);
            let (a_null, b_null) = (a.is_null(), b.is_null());
            // null/non-null discriminator: NULL first, never negated.
            match (a_null, b_null) {
                (true, true) => continue,
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                (false, false) => {}
            }
            let mut ord = compare_values(a, b);
            // descending negates only negatable (non-STRING) payloads.
            if descending[col_idx] && !matches!(a, Value::Str(_)) {
                ord = ord.reverse();
            }
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
    rows
}

/// Python `top_k_by_column` — sort indices by payload (optionally reversed),
/// take the first `k`. NULL ordering follows `compare_values` (NULL first),
/// reversed wholesale when `reverse` is set, matching Python's `reverse=True`.
pub fn top_k_by_column(values: &[Value], k: usize, reverse: bool) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..values.len()).collect();
    indices.sort_by(|&a, &b| {
        let ord = compare_values(&values[a], &values[b]);
        if reverse {
            ord.reverse()
        } else {
            ord
        }
    });
    indices.truncate(k);
    indices
}
