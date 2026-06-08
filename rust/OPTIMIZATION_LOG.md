# Optimization Log

Every row is measured: `cargo run --release -- bench --table events --iterations 5`
(50,000-row bootstrap demo, per-iteration average seconds). Correctness re-checked after
each step with `cargo test` (31 tests incl. differential-vs-Python). Revert on any regression.

## Profiling method
`cargo flamegraph` skipped: on macOS it needs `sudo dtrace` (interactive password prompt,
would hang headless). The `bench` harness already isolates per-operator time, and
ARCHITECTURE.md §3 pinpoints the within-function hotspots (value_fingerprint char-loop,
full-column codec decode, scan_column rebuild, chained-dict group-by, boxed sort keys).
Per-operator bench deltas are the optimization signal.

## Baseline — default release profile (lto=false, codegen-units=16)
| benchmark    | seconds |
|--------------|---------|
| groupby      | 0.0248  |
| sort         | 0.0198  |
| filter       | 0.0158  |
| hash_lookup  | 0.0051  |
| join         | 0.0019  |
| codec_slice  | 0.0006  |

Dominant: groupby > sort > filter. These are the Tier-2/3 targets.

## Steps
| # | Change | Tier | groupby | sort | filter | tests | keep? |
|---|--------|------|---------|------|--------|-------|-------|
| 0 | baseline | — | 0.0248 | 0.0198 | 0.0158 | 28 ✓ | — |
| 1 | release profile: lto=fat, codegen-units=1, panic=abort, target-cpu=native | 1 | 0.0220 | 0.0187 | 0.0138 | 28 ✓ | yes (−11% gb, −6% sort, −13% filt) |
| 2 | single-pass group-by (hashbrown key→accumulators, no 2nd pass/row-ids) | 2/3 | 0.0240 | — | — | 28 ✓ | **REVERTED** — −9% regression |

### Step 2 finding (reverted, but informs strategy)
`GROUP BY region` has only **5 distinct groups**, so the Python reference's "expensive"
two-pass design (store row-ids, re-scan per group via `apply_agg`) is already trivial here.
Single-pass added 2 Vec allocs + an extra key-string clone per row × 50k rows → net slower.
Single-pass *would* win on a high-cardinality group-by (e.g. GROUP BY user_id, ~tens of
thousands of groups) where the second pass dominates — but that's not what the bench measures.

**Real bottleneck (hypothesis):** all three hot benchmarks (groupby/filter/sort) run a
`SeqScan` that decodes **every column in the schema** (6 cols × 50k rows) into boxed `Value`s,
even when the query references only 2. The per-row group/filter/sort logic is cheap by
comparison. The high-leverage Tier-2 work is therefore **column pruning** (decode only
referenced columns) + a borrowing/typed scan that doesn't clone `String`s per cell — a shared
win across all three, and the genuine "de-box the hot path" lever from the playbook §5.2.
