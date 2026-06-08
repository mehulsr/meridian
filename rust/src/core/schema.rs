//! Table schema definitions and validation.
//!
//! Port of `meridian.core.schema`. Per the W1 warning: the `Schema` stores
//! `name -> usize` column indices (`HashMap<String, usize>`), **not** cloned
//! `ColumnDef`s — `ColumnDef`s live once in the `columns: Vec`.
//!
//! Python raises `ValueError`/`KeyError` here (not a `MeridianError` subclass);
//! we surface those as `MeridianError::Schema` so callers get a `Result`.

use std::collections::HashMap;

use crate::core::errors::{MeridianError, Result};
use crate::core::types::{coerce_to_type, DataType, Row, Value};

/// Port of Python `@dataclass ColumnDef`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    pub name: String,
    pub dtype: DataType,
    pub nullable: bool,
}

impl ColumnDef {
    /// Python `ColumnDef(name, dtype, nullable=True)`.
    pub fn new(name: impl Into<String>, dtype: DataType) -> Self {
        ColumnDef { name: name.into(), dtype, nullable: true }
    }

    /// Python `ColumnDef(name, dtype, nullable=...)`.
    pub fn with_nullable(name: impl Into<String>, dtype: DataType, nullable: bool) -> Self {
        ColumnDef { name: name.into(), dtype, nullable }
    }
}

/// Port of Python `@dataclass Schema`.
///
/// `index` mirrors Python's `_index` but stores the position in `columns`
/// (a `usize`) instead of a cloned `ColumnDef`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    pub columns: Vec<ColumnDef>,
    index: HashMap<String, usize>,
}

impl Schema {
    /// Python `Schema()` (empty).
    pub fn new() -> Self {
        Schema::default()
    }

    /// Python `Schema.add_column`.
    pub fn add_column(&mut self, col: ColumnDef) -> Result<()> {
        if self.index.contains_key(&col.name) {
            return Err(MeridianError::Schema(format!("duplicate column {}", col.name)));
        }
        self.index.insert(col.name.clone(), self.columns.len());
        self.columns.push(col);
        Ok(())
    }

    /// Python `Schema.get`. Missing column → Python `KeyError`.
    pub fn get(&self, name: &str) -> Result<&ColumnDef> {
        match self.index.get(name) {
            Some(&i) => Ok(&self.columns[i]),
            None => Err(MeridianError::Schema(format!("unknown column {name}"))),
        }
    }

    /// Python `Schema.has_column`.
    pub fn has_column(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    /// Python `Schema.column_names`.
    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }

    /// Python `Schema.validate_row` — width check + per-cell NULL/coercion.
    pub fn validate_row(&self, row: &Row) -> Result<Row> {
        if row.len() != self.columns.len() {
            return Err(MeridianError::Schema(format!(
                "row width {} != schema width {}",
                row.len(),
                self.columns.len()
            )));
        }
        let mut out: Vec<Value> = Vec::with_capacity(self.columns.len());
        for (col_def, value) in self.columns.iter().zip(row.iter()) {
            if value.is_null() {
                if !col_def.nullable {
                    return Err(MeridianError::Schema(format!(
                        "NULL in non-nullable column {}",
                        col_def.name
                    )));
                }
                out.push(Value::null(col_def.dtype));
                continue;
            }
            if value.data_type() != col_def.dtype {
                out.push(coerce_to_type(value, col_def.dtype));
            } else {
                out.push(value.clone());
            }
        }
        Ok(out)
    }

    /// Python `Schema.project_row` — reorder/select by column name.
    pub fn project_row(&self, row: &Row, names: &[String]) -> Row {
        names.iter().map(|n| row[self.index[n]].clone()).collect()
    }

    /// Python `Schema.from_pairs`.
    pub fn from_pairs(pairs: &[(String, DataType)]) -> Schema {
        let mut schema = Schema::new();
        for (name, dtype) in pairs {
            // from_pairs callers never pass duplicate names; unwrap mirrors that.
            schema.add_column(ColumnDef::new(name.clone(), *dtype)).unwrap();
        }
        schema
    }
}
