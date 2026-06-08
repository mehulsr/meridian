//! Port of meridian.ingest.batch_builder
//!
//! PRNG parity note (PLAYBOOK §3.3 / §5.0 gotcha): Python uses Mersenne-Twister
//! (random.Random). We do NOT attempt to reproduce it bit-for-bit; instead we use
//! a simple seeded LCG so tests can generate deterministic data independently.
//! For differential testing, generate data once in Python → CSV and load the same
//! CSV in both engines.

use crate::core::errors::Result;
use crate::core::types::Value;
use crate::storage::table::Table;

const REGIONS: &[&str] = &["us-east", "us-west", "eu-central", "ap-south", "ap-northeast"];
const CATEGORIES: &[&str] = &["ads", "subscription", "marketplace", "storage", "compute"];

/// Simple seeded LCG (Lehmer/Park-Miller) — NOT Mersenne-Twister.
/// Used only for synthetic batch generation; differential tests share a CSV.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        // Seed must be non-zero for this multiplier.
        Lcg { state: if seed == 0 { 1 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        // LCG: constants from Knuth TAOCP vol 2
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    /// Returns an integer in [lo, hi] inclusive.
    fn randint(&mut self, lo: i64, hi: i64) -> i64 {
        let range = (hi - lo + 1) as u64;
        (self.next_u64() % range) as i64 + lo
    }

    fn choice<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        let idx = (self.next_u64() as usize) % slice.len();
        &slice[idx]
    }

    /// Returns a float in [lo, hi) with 2 decimal places.
    fn uniform_2dp(&mut self, lo: f64, hi: f64) -> f64 {
        let frac = (self.next_u64() as f64) / (u64::MAX as f64);
        let raw = lo + frac * (hi - lo);
        (raw * 100.0).round() / 100.0
    }
}

pub fn generate_event_batch(table: &mut Table, start_id: i64, count: usize, seed: u64) -> Result<usize> {
    let mut rng = Lcg::new(seed);
    let mut batch: Vec<(String, Vec<Value>)> = vec![
        ("event_id".into(), Vec::with_capacity(count)),
        ("user_id".into(), Vec::with_capacity(count)),
        ("region".into(), Vec::with_capacity(count)),
        ("category".into(), Vec::with_capacity(count)),
        ("amount".into(), Vec::with_capacity(count)),
        ("ts".into(), Vec::with_capacity(count)),
    ];

    for i in 0..count as i64 {
        let eid = start_id + i;
        batch[0].1.push(Value::Int64(eid));
        batch[1].1.push(Value::Int64(rng.randint(1, 50000)));
        batch[2].1.push(Value::Str(rng.choice(REGIONS).to_string()));
        batch[3].1.push(Value::Str(rng.choice(CATEGORIES).to_string()));
        batch[4].1.push(Value::Float64(rng.uniform_2dp(0.5, 500.0)));
        batch[5].1.push(Value::Timestamp(1_700_000_000 + eid));
    }

    table.insert_column_batch(batch)
}
