//! General utilities.
//!
//! Port of `meridian.util.misc`. Python exposes a `timer` context manager plus
//! `chunked`/`flatten` list helpers. In Rust the context manager becomes a
//! closure-timing helper; the list helpers become generic functions.

use std::time::Instant;

/// Python `timer()` context manager — runs `f`, returns its result paired with
/// the elapsed wall-clock seconds (Python yielded a one-element bucket list that
/// received the `perf_counter` delta on exit).
pub fn timer<F, R>(f: F) -> (R, f64)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    (result, start.elapsed().as_secs_f64())
}

/// Python `chunked(iterable, size)` — split into contiguous slices of `size`
/// (last may be shorter). Materialized to owned `Vec`s.
pub fn chunked<T: Clone>(items: &[T], size: usize) -> Vec<Vec<T>> {
    items.chunks(size).map(|c| c.to_vec()).collect()
}

/// Python `flatten(nested)` — concatenate sub-lists in order.
pub fn flatten<T: Clone>(nested: &[Vec<T>]) -> Vec<T> {
    let mut out = Vec::new();
    for sub in nested {
        out.extend_from_slice(sub);
    }
    out
}
