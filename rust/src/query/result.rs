//! Query result container.
//!
//! Port of `meridian.query.result`. Holds a result set (column names + rows)
//! and provides formatting via `format_table` (exact column-padding logic).

use crate::core::types::Value;

/// Port of `ResultSet` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultSet {
    pub column_names: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl ResultSet {
    /// Python `ResultSet(column_names=[...])`.
    pub fn new(column_names: Vec<String>) -> ResultSet {
        ResultSet { column_names, rows: Vec::new() }
    }

    /// Python `ResultSet.add_row`.
    pub fn add_row(&mut self, row: Vec<Value>) {
        self.rows.push(row);
    }

    /// Python `ResultSet.__len__`.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Python `ResultSet.to_dicts` — convert rows to array of dicts (column → value).
    pub fn to_dicts(&self) -> Vec<std::collections::HashMap<String, Value>> {
        self.rows
            .iter()
            .map(|row| {
                self.column_names
                    .iter()
                    .zip(row.iter())
                    .map(|(name, val)| (name.clone(), val.clone()))
                    .collect()
            })
            .collect()
    }

    /// Python `ResultSet.format_table` — ASCII table with column padding.
    /// Max rows (default 50); appends "... (N more rows)" if truncated.
    pub fn format_table(&self, max_rows: usize) -> String {
        let mut widths: Vec<usize> = self.column_names.iter().map(|n| n.len()).collect();

        // Measure rendered cells up to max_rows
        let rendered: Vec<Vec<String>> = self
            .rows
            .iter()
            .take(max_rows)
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let cell = Self::fmt(v);
                        widths[i] = widths[i].max(cell.len());
                        cell
                    })
                    .collect()
            })
            .collect();

        let mut lines = Vec::new();

        // Header: names left-justified to width
        let header = self
            .column_names
            .iter()
            .enumerate()
            .map(|(i, name)| format!("{:<width$}", name, width = widths[i]))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(header);

        // Separator: dashes
        let sep = widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("-+-");
        lines.push(sep);

        // Rows: cells left-justified to width
        for cells in &rendered {
            let row = cells
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("{:<width$}", cell, width = widths[i]))
                .collect::<Vec<_>>()
                .join(" | ");
            lines.push(row);
        }

        // Overflow indicator
        if self.rows.len() > max_rows {
            lines.push(format!("... ({} more rows)", self.rows.len() - max_rows));
        }

        lines.join("\n")
    }

    /// Python `ResultSet._fmt` — format a `Value` for display.
    fn fmt(v: &Value) -> String {
        if v.is_null() {
            "NULL".to_string()
        } else {
            match v {
                Value::Null(_) => "NULL".to_string(),
                Value::Int64(x) => x.to_string(),
                Value::Float64(x) => x.to_string(),
                Value::Str(s) => s.clone(),
                Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
                Value::Timestamp(t) => t.to_string(),
            }
        }
    }
}
