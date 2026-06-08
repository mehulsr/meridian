//! Port of meridian.bench.runner — benchmark harness for the 6 named benchmarks.

use std::time::Instant;

use crate::core::catalog::Catalog;
use crate::core::errors::Result;
use crate::core::schema::{ColumnDef, Schema};
use crate::core::types::{DataType, Value};
use crate::index::hash_index::HashIndex;
use crate::query::executor::Executor;
use crate::query::parser::parse_query;
use crate::query::planner::Planner;
use crate::storage::table::{IndexKind, Table};

/// Execute SQL without cloning the catalog on every call.
/// Planner and Executor each clone internally; we clone once here per benchmark call.
fn run_sql(catalog: &Catalog, sql: &str) {
    let Ok(q) = parse_query(sql) else { return };
    let Ok(plan) = Planner::new(catalog.clone()).plan(&q) else { return };
    let _ = Executor::new(catalog.clone()).execute(&plan);
}

pub fn bench_groupby_region(catalog: &Catalog, table: &str) {
    run_sql(
        catalog,
        &format!("SELECT region, COUNT(*) AS n, SUM(amount) AS revenue FROM {table} GROUP BY region"),
    );
}

pub fn bench_filter_project(catalog: &Catalog, table: &str) {
    run_sql(catalog, &format!("SELECT event_id, user_id, amount FROM {table} WHERE amount > 100"));
}

pub fn bench_order_by_limit(catalog: &Catalog, table: &str) {
    run_sql(catalog, &format!("SELECT user_id, amount FROM {table} ORDER BY amount DESC LIMIT 100"));
}

pub fn bench_hash_lookup(catalog: &Catalog, table: &str) {
    let t = match catalog.get_table(table) {
        Ok(t) => t,
        Err(_) => return,
    };
    let col = t.scan_column("user_id");
    let mut idx = HashIndex::new("user_id");
    idx.build(&col);
    for uid in 1i64..=5000 {
        idx.lookup(&Value::Int64(uid));
    }
}

pub fn bench_codec_decode_slice(catalog: &Catalog, table: &str) {
    let t = match catalog.get_table(table) {
        Ok(t) => t,
        Err(_) => return,
    };
    if t.chunk_count(Some("amount")) == 0 {
        return;
    }
    let chunk = t.get_chunk("amount", 0);
    let codec = chunk.codec_name;
    let rc = chunk.row_count;
    let payload = chunk.payload.clone();
    let dtype = chunk.dtype;
    let mut start = 0usize;
    while start < rc.min(8000) {
        let end = (start + 64).min(rc);
        codec.decode_range(&payload, dtype, start, end);
        start += 128;
    }
}

pub fn bench_join_user_events(catalog: &mut Catalog, table: &str) {
    if !catalog.has_table("users") {
        ensure_users(catalog);
    }
    run_sql(
        catalog,
        &format!("SELECT region, COUNT(*) AS n FROM {table} INNER JOIN users ON user_id = id GROUP BY region"),
    );
}

fn ensure_users(catalog: &mut Catalog) {
    let mut schema = Schema::new();
    let _ = schema.add_column(ColumnDef::new("id", DataType::Int64));
    let _ = schema.add_column(ColumnDef::new("tier", DataType::String));
    let mut users = Table::new("users", schema);
    let ids: Vec<Value> = (1i64..=50000).map(Value::Int64).collect();
    let tiers: Vec<Value> =
        (1i64..=50000).map(|i| Value::Str(if i % 3 == 0 { "pro" } else { "free" }.into())).collect();
    let _ = users.insert_column_batch(vec![("id".into(), ids), ("tier".into(), tiers)]);
    users.finalize_chunks();
    let _ = catalog.register_table(users);
}

fn timeit<F: Fn()>(f: &F, iterations: usize) -> f64 {
    let iters = iterations.max(1);
    let start = Instant::now();
    for _ in 0..iters {
        f();
    }
    start.elapsed().as_secs_f64() / iters as f64
}

pub fn run_benchmarks(
    catalog: &mut Catalog,
    table: &str,
    iterations: usize,
    filter_name: Option<&str>,
) -> Result<Vec<(String, f64)>> {
    let names: &[&str] = &["groupby", "filter", "sort", "hash_lookup", "codec_slice", "join"];
    // Ensure users table exists before any join benchmark
    ensure_users(catalog);

    let mut results = Vec::new();
    for &name in names {
        if let Some(f) = filter_name {
            if name != f {
                continue;
            }
        }
        // Snapshot catalog once per benchmark — not inside the hot loop
        let cat = catalog.clone();
        let t = table.to_string();
        let elapsed = match name {
            "groupby"     => timeit(&|| bench_groupby_region(&cat, &t), iterations),
            "filter"      => timeit(&|| bench_filter_project(&cat, &t), iterations),
            "sort"        => timeit(&|| bench_order_by_limit(&cat, &t), iterations),
            "hash_lookup" => timeit(&|| bench_hash_lookup(&cat, &t), iterations),
            "codec_slice" => timeit(&|| bench_codec_decode_slice(&cat, &t), iterations),
            "join"        => timeit(&|| {
                run_sql(&cat,
                    &format!("SELECT region, COUNT(*) AS n FROM {t} INNER JOIN users ON user_id = id GROUP BY region"));
            }, iterations),
            _ => continue,
        };
        results.push((name.to_string(), elapsed));
    }
    Ok(results)
}
