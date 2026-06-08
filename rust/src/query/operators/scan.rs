//! Sequential table scan.
//!
//! Port of `meridian.query.operators.scan`. Materializes all columns from a
//! table into a dict-of-columns view.

use std::collections::HashMap;

use crate::core::catalog::Catalog;
use crate::core::errors::Result;
use crate::storage::column::Column;

/// Port of Python's `scan_table` function.
pub fn scan_table(catalog: &Catalog, table_name: &str) -> Result<HashMap<String, Column>> {
    let table = catalog.get_table(table_name)?;
    let mut columns = HashMap::new();
    for name in table.schema.column_names() {
        columns.insert(name.clone(), table.scan_column(&name));
    }
    Ok(columns)
}
