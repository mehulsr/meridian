//! Catalog and database-level metadata.
//!
//! Port of `meridian.core.catalog`. In Python the `Table` import is behind
//! `TYPE_CHECKING`; in Rust it is a normal dependency (no cycle). `list_tables`
//! sorts keys on read (per the W3 warning) rather than storing a `BTreeMap`.

use std::collections::HashMap;

use crate::core::errors::{MeridianError, Result};
use crate::core::schema::Schema;
use crate::storage::table::Table;

/// Port of the `@dataclass Catalog`.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub tables: HashMap<String, Table>,
}

impl Catalog {
    /// Python `Catalog()`.
    pub fn new() -> Catalog {
        Catalog::default()
    }

    /// Python `Catalog.register_table` — duplicate name is a `ValueError`.
    pub fn register_table(&mut self, table: Table) -> Result<()> {
        if self.tables.contains_key(&table.name) {
            return Err(MeridianError::Schema(format!(
                "table {} already exists",
                table.name
            )));
        }
        self.tables.insert(table.name.clone(), table);
        Ok(())
    }

    /// Python `Catalog.get_table` — missing name is a `KeyError`.
    pub fn get_table(&self, name: &str) -> Result<&Table> {
        self.tables
            .get(name)
            .ok_or_else(|| MeridianError::Schema(format!("table {name} not found")))
    }

    /// Mutable accessor (Python returns the same mutable object from the dict).
    pub fn get_table_mut(&mut self, name: &str) -> Result<&mut Table> {
        self.tables
            .get_mut(name)
            .ok_or_else(|| MeridianError::Schema(format!("table {name} not found")))
    }

    /// Python `Catalog.has_table`.
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// Python `Catalog.drop_table` — `del` raises `KeyError` if absent.
    pub fn drop_table(&mut self, name: &str) {
        self.tables
            .remove(name)
            .unwrap_or_else(|| panic!("{name}"));
    }

    /// Python `Catalog.list_tables` — `sorted(self.tables.keys())`.
    pub fn list_tables(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tables.keys().cloned().collect();
        names.sort();
        names
    }

    /// Python `Catalog.table_schema`.
    pub fn table_schema(&self, name: &str) -> Result<&Schema> {
        self.get_table(name).map(|t| &t.schema)
    }

    /// Python `Catalog.total_rows`.
    pub fn total_rows(&self) -> usize {
        self.tables.values().map(|t| t.row_count()).sum()
    }

    /// Python `Catalog.find_table_with_column` — first table with the column.
    /// Python iterates dict insertion order; with a `HashMap` order is arbitrary,
    /// matching the Python contract of "some matching table" (any is acceptable).
    pub fn find_table_with_column(&self, column: &str) -> Option<String> {
        self.tables
            .iter()
            .find(|(_, t)| t.schema.has_column(column))
            .map(|(name, _)| name.clone())
    }
}
