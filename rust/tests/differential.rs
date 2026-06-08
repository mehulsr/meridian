//! Differential test: Rust engine vs Python engine.
//!
//! Loads the SAME data/sample_events.csv that scripts/gen_golden.py used, runs the
//! SAME SQL battery, and compares row-sets against rust/tests/golden/diff.json.
//! Order-insensitive queries are sorted by an identical canonical key on both sides;
//! floats compared with tolerance. Regenerate the golden with:
//!     python scripts/gen_golden.py

use std::path::PathBuf;

use meridian::core::types::{DataType, Value};
use meridian::ingest::csv_reader::load_csv;
use meridian::query::result::ResultSet;
use meridian::runtime::session::Session;
use serde_json::Value as J;

const FLOAT_TOL: f64 = 1e-6;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/rust
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Convert one engine cell to the typed JSON shape gen_golden.py emits.
fn cell_to_json(v: &Value) -> J {
    match v {
        Value::Null(_) => serde_json::json!({ "t": "null" }),
        Value::Int64(i) => serde_json::json!({ "t": "int", "v": i }),
        Value::Timestamp(i) => serde_json::json!({ "t": "ts", "v": i }),
        Value::Float64(f) => serde_json::json!({ "t": "float", "v": f }),
        Value::Bool(b) => serde_json::json!({ "t": "bool", "v": b }),
        Value::Str(s) => serde_json::json!({ "t": "str", "v": s }),
    }
}

/// Identical canonical sort key to gen_golden.py::canon_key.
fn canon_key(row: &[J]) -> String {
    let mut parts = Vec::with_capacity(row.len());
    for c in row {
        let t = c["t"].as_str().unwrap();
        match t {
            "null" => parts.push("null".to_string()),
            "float" => {
                let f = c["v"].as_f64().unwrap();
                parts.push(format!("float:{:.6}", (f * 1e6).round() / 1e6));
            }
            "str" => parts.push(format!("str:{}", c["v"].as_str().unwrap())),
            "bool" => parts.push(format!("bool:{}", c["v"].as_bool().unwrap())),
            _ => parts.push(format!("{}:{}", t, c["v"].as_i64().unwrap())),
        }
    }
    parts.join("|")
}

fn cells_eq(a: &J, b: &J) -> bool {
    if a["t"] != b["t"] {
        return false;
    }
    match a["t"].as_str().unwrap() {
        "null" => true,
        "float" => (a["v"].as_f64().unwrap() - b["v"].as_f64().unwrap()).abs() <= FLOAT_TOL,
        "str" => a["v"].as_str() == b["v"].as_str(),
        "bool" => a["v"].as_bool() == b["v"].as_bool(),
        _ => a["v"].as_i64() == b["v"].as_i64(),
    }
}

fn result_to_rows(res: &ResultSet) -> Vec<Vec<J>> {
    res.rows.iter().map(|row| row.iter().map(cell_to_json).collect()).collect()
}

#[test]
fn differential_matches_python() {
    let root = repo_root();
    let csv = root.join("data/sample_events.csv");
    let golden_path = root.join("rust/tests/golden/diff.json");

    let golden: J =
        serde_json::from_str(&std::fs::read_to_string(&golden_path).expect("read golden"))
            .expect("parse golden");

    // One catalog/session, load the identical CSV once.
    let mut session = Session::new();
    let table = load_csv(&csv, "events", None, 5000).expect("load_csv");
    session.catalog.register_table(table).expect("register");

    let queries = golden["queries"].as_array().expect("queries array");
    let mut checked = 0usize;

    for entry in queries {
        let sql = entry["sql"].as_str().unwrap();
        let order_sensitive = entry["order_sensitive"].as_bool().unwrap();

        let res = session.sql(sql).unwrap_or_else(|e| panic!("rust sql failed: {sql}\n{e}"));

        // Column names must match.
        let got_cols: Vec<&str> = res.column_names.iter().map(|s| s.as_str()).collect();
        let exp_cols: Vec<&str> =
            entry["column_names"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
        assert_eq!(got_cols, exp_cols, "column names differ for: {sql}");

        let mut got = result_to_rows(&res);
        let mut exp: Vec<Vec<J>> = entry["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_array().unwrap().clone())
            .collect();

        assert_eq!(got.len(), exp.len(), "row count differs for: {sql}");

        if !order_sensitive {
            got.sort_by_key(|r| canon_key(r));
            exp.sort_by_key(|r| canon_key(r));
        }

        for (i, (g, e)) in got.iter().zip(exp.iter()).enumerate() {
            assert_eq!(g.len(), e.len(), "row width differs at {i} for: {sql}");
            for (j, (gc, ec)) in g.iter().zip(e.iter()).enumerate() {
                assert!(
                    cells_eq(gc, ec),
                    "cell mismatch at row {i} col {j} for: {sql}\n  rust={gc}\n  py={ec}"
                );
            }
        }
        checked += 1;
    }

    assert_eq!(checked, queries.len());
    // Touch DataType to keep the import meaningful if cells ever need typing.
    let _ = DataType::Int64;
}
