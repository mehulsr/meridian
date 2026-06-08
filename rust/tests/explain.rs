//! Port of `tests/test_explain.py` (W9: query::explain).

use meridian::core::catalog::Catalog;
use meridian::ingest::batch_builder::generate_event_batch;
use meridian::ingest::json_reader::events_schema;
use meridian::query::explain::explain_node;
use meridian::query::parser::parse_query;
use meridian::query::planner::Planner;
use meridian::storage::table::Table;

#[test]
fn test_explain_plan_contains_scan() {
    let mut catalog = Catalog::new();
    let mut table = Table::new("events", events_schema());
    generate_event_batch(&mut table, 1, 100, 1).expect("batch generation failed");
    catalog.register_table(table).expect("register failed");

    let q = parse_query("SELECT region FROM events WHERE amount > 10")
        .expect("parse failed");
    let plan = Planner::new(catalog).plan(&q).expect("plan failed");
    let text = explain_node(&plan.root, 0);

    assert!(text.contains("SeqScan"), "expected 'SeqScan' in: {}", text);
    assert!(text.contains("Filter"), "expected 'Filter' in: {}", text);
}
