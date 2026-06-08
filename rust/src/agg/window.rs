//! Port of meridian.agg.window — extension point, unwired in live path.

use crate::core::types::Value;
use crate::storage::column::Column;

pub struct WindowFrame {
    pub rows: Vec<usize>,
}

impl WindowFrame {
    pub fn new(rows: Vec<usize>) -> Self {
        WindowFrame { rows }
    }

    pub fn row_number(&self) -> Vec<Value> {
        (1..=self.rows.len()).map(|i| Value::Int64(i as i64)).collect()
    }

    pub fn running_sum(&self, column: &Column) -> Vec<Value> {
        let mut total = 0.0f64;
        let mut out = Vec::with_capacity(self.rows.len());
        for &r in &self.rows {
            let v = column.get(r);
            if !v.is_null() {
                total += match v {
                    Value::Int64(i) => i as f64,
                    Value::Float64(f) => f,
                    _ => 0.0,
                };
            }
            out.push(Value::Float64(total));
        }
        out
    }
}
