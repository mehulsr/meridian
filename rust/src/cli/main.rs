//! Port of meridian.cli.main — columnar analytics engine CLI.
//!
//! Subcommands: ingest, generate, query, bench, tables.
//! Uses clap derive macros mirroring the Python argparse setup.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::bench::runner::run_benchmarks;
use crate::core::catalog::Catalog;
use crate::index::bloom::BloomFilter;
use crate::index::hash_index::HashIndex;
use crate::ingest::batch_builder::generate_event_batch;
use crate::ingest::csv_reader::load_csv;
use crate::ingest::json_reader::{events_schema, load_jsonl};
use crate::query::executor::Executor;
use crate::query::parser::parse_query;
use crate::query::planner::Planner;
use crate::storage::table::{IndexKind, Table};

#[derive(Parser)]
#[command(name = "meridian", about = "Columnar analytics engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Load CSV/JSONL into a table
    Ingest {
        path: PathBuf,
        #[arg(long, default_value = "events")]
        table: String,
        #[arg(long, default_value = "user_id,region")]
        build_index: String,
    },
    /// Generate synthetic events
    Generate {
        #[arg(long, default_value = "events")]
        table: String,
        #[arg(long, default_value_t = 100000)]
        rows: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
    },
    /// Run a SQL-ish query
    Query {
        sql: String,
        #[arg(long, default_value = "events")]
        table: String,
        #[arg(long, default_value_t = 50)]
        max_rows: usize,
    },
    /// Run benchmarks
    Bench {
        #[arg(long, default_value = "events")]
        table: String,
        #[arg(long, default_value_t = 3)]
        iterations: usize,
        #[arg(long)]
        filter: Option<String>,
    },
    /// List tables
    Tables,
}

pub fn run() {
    let cli = Cli::parse();
    let mut catalog = Catalog::new();
    let rc = dispatch(&mut catalog, cli.command);
    std::process::exit(rc);
}

fn dispatch(catalog: &mut Catalog, cmd: Commands) -> i32 {
    match cmd {
        Commands::Ingest { path, table, build_index } => cmd_ingest(catalog, &path, &table, &build_index),
        Commands::Generate { table, rows, seed } => cmd_generate(catalog, &table, rows, seed),
        Commands::Query { sql, table, max_rows } => cmd_query(catalog, &sql, &table, max_rows),
        Commands::Bench { table, iterations, filter } => {
            cmd_bench(catalog, &table, iterations, filter.as_deref())
        }
        Commands::Tables => cmd_tables(catalog),
    }
}

fn cmd_ingest(catalog: &mut Catalog, path: &PathBuf, table_name: &str, build_index: &str) -> i32 {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut table = match ext {
        "csv" => match load_csv(path, table_name, None, 5000) {
            Ok(t) => t,
            Err(e) => { eprintln!("error: {e}"); return 1; }
        },
        "jsonl" | "ndjson" => match load_jsonl(path, table_name, &events_schema()) {
            Ok(t) => t,
            Err(e) => { eprintln!("error: {e}"); return 1; }
        },
        _ => { eprintln!("unsupported format: .{ext}"); return 1; }
    };
    for col_name in build_index.split(',') {
        let col_name = col_name.trim();
        let mut idx = HashIndex::new(col_name);
        idx.build(&table.scan_column(col_name));
        table.attach_index(col_name, IndexKind::Hash(idx));
    }
    let mut bloom = BloomFilter::new(table.row_count(), 0.01);
    bloom.build(&table.scan_column("region"));
    table.attach_index("region_bloom", IndexKind::Bloom(bloom));
    let stats = table.stats();
    let rows = stats.rows;
    let chunks = stats.chunks;
    let encoded = stats.encoded_bytes;
    println!("ingested {rows} rows into {table_name} ({chunks} chunks)");
    let _ = catalog.register_table(table);
    0
}

fn cmd_generate(catalog: &mut Catalog, table_name: &str, rows: usize, seed: u64) -> i32 {
    let schema = events_schema();
    let mut table = if catalog.has_table(table_name) {
        catalog.get_table(table_name).unwrap().clone()
    } else {
        Table::new(table_name, schema)
    };
    let start = table.row_count() as i64 + 1;
    match generate_event_batch(&mut table, start, rows, seed) {
        Ok(inserted) => {
            table.finalize_chunks();
            let _ = catalog.register_table(table);
            println!("generated {inserted} rows in {table_name}");
            0
        }
        Err(e) => { eprintln!("error: {e}"); 1 }
    }
}

fn cmd_query(catalog: &mut Catalog, sql: &str, table_name: &str, max_rows: usize) -> i32 {
    if !catalog.has_table(table_name) {
        bootstrap_demo(catalog, 5000);
    }
    let query = match parse_query(sql) {
        Ok(q) => q,
        Err(e) => { eprintln!("parse error: {e}"); return 1; }
    };
    let plan = match Planner::new(catalog.clone()).plan(&query) {
        Ok(p) => p,
        Err(e) => { eprintln!("plan error: {e}"); return 1; }
    };
    match Executor::new(catalog.clone()).execute(&plan) {
        Ok(result) => { println!("{}", result.format_table(max_rows)); 0 }
        Err(e) => { eprintln!("execute error: {e}"); 1 }
    }
}

fn cmd_bench(
    catalog: &mut Catalog,
    table_name: &str,
    iterations: usize,
    filter: Option<&str>,
) -> i32 {
    if !catalog.has_table(table_name) {
        bootstrap_demo(catalog, 500000);
    }
    match run_benchmarks(catalog, table_name, iterations, filter) {
        Ok(results) => {
            for (name, secs) in &results {
                println!("{name:<30} {secs:.4}s");
            }
            0
        }
        Err(e) => { eprintln!("bench error: {e}"); 1 }
    }
}

fn cmd_tables(catalog: &Catalog) -> i32 {
    if catalog.list_tables().is_empty() {
        println!("no tables");
        return 0;
    }
    for name in catalog.list_tables() {
        if let Ok(t) = catalog.get_table(&name) {
            let s = t.stats();
            println!(
                "{name:<20} rows={} chunks={} encoded={}B",
                s.rows, s.chunks, s.encoded_bytes
            );
        }
    }
    0
}

fn bootstrap_demo(catalog: &mut Catalog, rows: usize) {
    let schema = events_schema();
    let mut table = Table::with_chunk_size("events", schema, 4096);
    let _ = generate_event_batch(&mut table, 1, rows, 42);
    table.finalize_chunks();
    let mut idx = HashIndex::new("user_id");
    idx.build(&table.scan_column("user_id"));
    table.attach_index("user_id", IndexKind::Hash(idx));
    let _ = catalog.register_table(table);
}
