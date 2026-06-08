//! Table and column statistics for the cost model.
//!
//! Port of `meridian.query.optimizer.stats`. **Unwired** — minimal parity port.
//! Collects basic stats (row/null/distinct counts, min/max, avg width) for cost estimation.


use crate::core::types::Value;
use crate::storage::column::Column;
use crate::storage::table::Table;

/// Port of `ColumnStats` dataclass.
#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub name: String,
    pub row_count: usize,
    pub null_count: usize,
    pub distinct_estimate: usize,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub avg_width: f64,
}

/// Port of `TableStats` dataclass.
#[derive(Debug, Clone)]
pub struct TableStats {
    pub table_name: String,
    pub row_count: usize,
    pub columns: std::collections::HashMap<String, ColumnStats>,
    pub total_bytes_estimate: usize,
}

impl TableStats {
    pub fn new(table_name: impl Into<String>, row_count: usize) -> TableStats {
        TableStats {
            table_name: table_name.into(),
            row_count,
            columns: std::collections::HashMap::new(),
            total_bytes_estimate: 0,
        }
    }
}

/// Port of `StatsCollector` class.
pub struct StatsCollector;

impl StatsCollector {
    /// Collect stats for an entire table.
    pub fn collect_table(&self, table: &Table) -> TableStats {
        let mut stats = TableStats::new(table.name.clone(), table.row_count());
        for col_def in &table.schema.columns {
            let col = table.scan_column(&col_def.name);
            stats.columns.insert(col_def.name.clone(), self.collect_column(&col));
        }
        stats.total_bytes_estimate = stats
            .columns
            .values()
            .map(|c| (c.row_count as f64 * c.avg_width) as usize)
            .sum();
        stats
    }

    /// Collect stats for a single column.
    pub fn collect_column(&self, column: &Column) -> ColumnStats {
        let mut nulls = 0;
        let mut seen: Vec<Value> = Vec::new();
        let mut min_v: Option<Value> = None;
        let mut max_v: Option<Value> = None;
        let mut width_total = 0usize;

        for i in 0..column.len() {
            let v = column.get(i);
            if v.is_null() {
                nulls += 1;
                continue;
            }
            if !seen.contains(&v) {
                seen.push(v.clone());
            }
            width_total += format!("{:?}", v).len();
            if min_v.is_none() {
                min_v = Some(v.clone());
                max_v = Some(v);
            } else {
                use std::cmp::Ordering;
                use crate::core::types::compare_values;
                if compare_values(&v, min_v.as_ref().unwrap()) == Ordering::Less {
                    min_v = Some(v.clone());
                }
                if compare_values(&v, max_v.as_ref().unwrap()) == Ordering::Greater {
                    max_v = Some(v);
                }
            }
        }
        let n = column.len();
        let avg_width = if n > nulls {
            width_total as f64 / (n - nulls) as f64
        } else {
            8.0
        };
        ColumnStats {
            name: column.name.clone(),
            row_count: n,
            null_count: nulls,
            distinct_estimate: seen.len(),
            min_value: min_v,
            max_value: max_v,
            avg_width,
        }
    }
}
