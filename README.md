# Meridian

Goal: Learn Rust.

Meridian is a columnar in-memory analytics engine written in Rust, ported from its Python implementation. It ingests event batches, stores them in compressed column chunks, indexes hot columns, and executes a small SQL-like query language over the data.


## Quick start

Compared to original Python code runs.

```bash
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
meridian ingest data/sample_events.csv --table events
meridian query "SELECT region, COUNT(*) AS n FROM events WHERE amount > 10 GROUP BY region ORDER BY n DESC LIMIT 5"
meridian bench --table events --iterations 3
python -m pytest tests/ -q
```

## Layout

Original Python code layout.

```
src/meridian/
  core/       types, schema, arena allocatorV
  storage/    column chunks, codecs, table catalog
  index/      hash, btree, bloom, inverted indexes
  query/      parser, planner, executor, operators
  ingest/     CSV/JSON readers, batch builder
  agg/        aggregate functions, group-by, windows
  util/       hashing, sorting, bitmaps
  cli/        command-line interface
  bench/      benchmark harness
```

## Performance notes

The reference implementation favors clarity over speed. Known slow paths:

- Group-by uses nested dicts instead of open-addressing hash tables
- Joins use nested-loop with repeated column lookups
- Sort uses Python's Timsort on boxed Python objects
- Codecs decode entire columns even when only a slice is needed
- Bloom filters re-hash strings on every probe

These are deliberate targets for optimization and Rust porting.
