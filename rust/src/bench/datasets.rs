//! Port of meridian.bench.datasets — synthetic dataset helpers.
//!
//! PRNG note: uses the same seeded LCG as ingest::batch_builder (not MT).
//! For differential testing, generate data in Python → CSV and load that CSV.

use std::io::Write;
use std::path::Path;

use crate::core::errors::{MeridianError, Result};

const REGIONS: &[&str] = &["us-east", "us-west", "eu-central", "ap-south", "ap-northeast"];
const CATEGORIES: &[&str] = &["ads", "subscription", "marketplace", "storage", "compute"];

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg { state: if seed == 0 { 1 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        self.state =
            self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn randint(&mut self, lo: i64, hi: i64) -> i64 {
        (self.next_u64() % (hi - lo + 1) as u64) as i64 + lo
    }

    fn choice<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[(self.next_u64() as usize) % slice.len()]
    }

    fn uniform_2dp(&mut self, lo: f64, hi: f64) -> f64 {
        let frac = self.next_u64() as f64 / u64::MAX as f64;
        ((lo + frac * (hi - lo)) * 100.0).round() / 100.0
    }
}

/// Write a synthetic events CSV to `path` with `rows` rows.
pub fn write_sample_csv(path: &Path, rows: usize, seed: u64) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| MeridianError::Ingest(e.to_string()))?;
    }
    let mut f =
        std::fs::File::create(path).map_err(|e| MeridianError::Ingest(e.to_string()))?;
    writeln!(f, "event_id,user_id,region,category,amount,ts")
        .map_err(|e| MeridianError::Ingest(e.to_string()))?;
    let mut rng = Lcg::new(seed);
    for i in 1..=(rows as i64) {
        writeln!(
            f,
            "{},{},{},{},{:.2},{}",
            i,
            rng.randint(1, 5000),
            rng.choice(REGIONS),
            rng.choice(CATEGORIES),
            rng.uniform_2dp(1.0, 250.0),
            1_700_000_000i64 + i,
        )
        .map_err(|e| MeridianError::Ingest(e.to_string()))?;
    }
    Ok(())
}
