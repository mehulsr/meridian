"""Generate golden query results from the Python meridian engine.

Runs a battery of SQL queries against data/sample_events.csv and dumps typed,
canonicalized results to rust/tests/golden/diff.json. The Rust differential test
loads the SAME csv, runs the SAME sql, and compares row-sets.

Cells are emitted typed so the Rust side can compare with float tolerance and
sort order-insensitive queries by an identical canonical key.
"""

from __future__ import annotations

import json
from pathlib import Path

from meridian.core.catalog import Catalog
from meridian.ingest.csv_reader import load_csv
from meridian.query.executor import Executor
from meridian.query.parser import parse_query
from meridian.query.planner import Planner

ROOT = Path(__file__).resolve().parent.parent
CSV = ROOT / "data" / "sample_events.csv"
OUT = ROOT / "rust" / "tests" / "golden" / "diff.json"

# (sql, order_sensitive). order_sensitive=True keeps row order (ORDER BY/LIMIT);
# False means the row-set is compared order-insensitively.
QUERIES = [
    ("SELECT region, COUNT(*) AS n, SUM(amount) AS rev FROM events GROUP BY region", False),
    ("SELECT region, COUNT(*) AS n, MIN(amount) AS lo, MAX(amount) AS hi FROM events GROUP BY region", False),
    ("SELECT category, AVG(amount) AS mean FROM events GROUP BY category", False),
    ("SELECT event_id, user_id, amount FROM events WHERE amount > 100 AND region = 'us-east'", False),
    ("SELECT user_id, amount FROM events ORDER BY amount DESC LIMIT 100", True),
    ("SELECT region, category, amount FROM events ORDER BY amount ASC LIMIT 50", True),
    ("SELECT amount FROM events WHERE amount > 100 LIMIT 5", False),
]


def cell(v) -> dict:
    if v.is_null():
        return {"t": "null"}
    name = v.type.name
    if name == "INT64":
        return {"t": "int", "v": int(v.payload)}
    if name == "TIMESTAMP":
        return {"t": "ts", "v": int(v.payload)}
    if name == "FLOAT64":
        return {"t": "float", "v": float(v.payload)}
    if name == "BOOL":
        return {"t": "bool", "v": bool(v.payload)}
    return {"t": "str", "v": str(v.payload)}


def canon_key(row: list[dict]) -> str:
    parts = []
    for c in row:
        if c["t"] == "null":
            parts.append("null")
        elif c["t"] == "float":
            parts.append(f"float:{round(c['v'], 6):.6f}")
        else:
            parts.append(f"{c['t']}:{c['v']}")
    return "|".join(parts)


def main() -> None:
    catalog = Catalog()
    catalog.register_table(load_csv(str(CSV), "events"))

    entries = []
    for sql, order_sensitive in QUERIES:
        plan = Planner(catalog).plan(parse_query(sql))
        res = Executor(catalog).execute(plan)
        rows = [[cell(v) for v in row] for row in res.rows]
        if not order_sensitive:
            rows.sort(key=canon_key)
        entries.append(
            {
                "sql": sql,
                "order_sensitive": order_sensitive,
                "column_names": list(res.column_names),
                "rows": rows,
            }
        )

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps({"queries": entries}, indent=2))
    print(f"wrote {len(entries)} golden queries to {OUT}")
    for e in entries:
        print(f"  {len(e['rows']):>5} rows  {e['sql'][:70]}")


if __name__ == "__main__":
    main()
