//! Port of meridian.bench.profile_guide — profiling helpers.
//!
//! cProfile / pstats → no direct Rust equivalent. We expose `time_call`
//! (measures wall time for a closure) and the PROFILE_TARGETS map as constants.

use std::time::Instant;

/// Time a closure over `iterations` calls; return (avg_seconds, last_result).
pub fn time_call<F, T>(f: F, iterations: usize) -> (f64, T)
where
    F: Fn() -> T,
{
    let iters = iterations.max(1);
    let start = Instant::now();
    let mut result = f();
    for _ in 1..iters {
        result = f();
    }
    (start.elapsed().as_secs_f64() / iters as f64, result)
}

/// Symbolic names of the hot-path targets (mirrors Python PROFILE_TARGETS dict).
pub const PROFILE_TARGETS: &[(&str, &str)] = &[
    ("groupby", "meridian::agg::groupby::GroupByEngine::aggregate"),
    ("filter", "meridian::query::operators::filter::filter_rows"),
    ("codec", "meridian::storage::codec::base::DeltaCodec::decode_range"),
    ("scan", "meridian::storage::table::Table::scan_column"),
    ("hash", "meridian::index::hash_index::HashIndex::lookup"),
];
