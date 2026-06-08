//! Port of meridian.runtime.metrics — operator-level timing metrics.

use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct OperatorMetric {
    pub name: String,
    pub calls: u64,
    pub total_ms: f64,
    pub rows_processed: u64,
}

impl OperatorMetric {
    pub fn new(name: impl Into<String>) -> Self {
        OperatorMetric { name: name.into(), calls: 0, total_ms: 0.0, rows_processed: 0 }
    }

    pub fn record(&mut self, elapsed_ms: f64, rows: u64) {
        self.calls += 1;
        self.total_ms += elapsed_ms;
        self.rows_processed += rows;
    }

    pub fn avg_ms(&self) -> f64 {
        self.total_ms / self.calls.max(1) as f64
    }
}

#[derive(Debug, Default)]
pub struct MetricsRegistry {
    operators: HashMap<String, OperatorMetric>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        MetricsRegistry::default()
    }

    pub fn get_or_create(&mut self, name: &str) -> &mut OperatorMetric {
        self.operators.entry(name.to_string()).or_insert_with(|| OperatorMetric::new(name))
    }

    pub fn time<F, T>(&mut self, name: &str, rows: u64, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.get_or_create(name).record(elapsed_ms, rows);
        result
    }

    pub fn report(&self) -> Vec<MetricRow> {
        let mut rows: Vec<&OperatorMetric> = self.operators.values().collect();
        rows.sort_by(|a, b| {
            b.total_ms.partial_cmp(&a.total_ms).unwrap_or(std::cmp::Ordering::Equal)
        });
        rows.iter()
            .map(|m| MetricRow {
                operator: m.name.clone(),
                calls: m.calls,
                total_ms: (m.total_ms * 1000.0).round() / 1000.0,
                avg_ms: (m.avg_ms() * 1000.0).round() / 1000.0,
                rows: m.rows_processed,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct MetricRow {
    pub operator: String,
    pub calls: u64,
    pub total_ms: f64,
    pub avg_ms: f64,
    pub rows: u64,
}
