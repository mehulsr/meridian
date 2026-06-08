//! Port of tests/test_integration.py — Session.sql end-to-end.

use meridian::ingest::batch_builder::generate_event_batch;
use meridian::ingest::json_reader::events_schema;
use meridian::runtime::session::Session;
use meridian::storage::table::Table;

fn make_session(rows: usize, seed: u64) -> Session {
    let mut session = Session::new();
    let schema = events_schema();
    let mut table = Table::with_chunk_size("events", schema, 256);
    generate_event_batch(&mut table, 1, rows, seed).unwrap();
    table.finalize_chunks();
    session.catalog.register_table(table).unwrap();
    session
}

#[test]
fn test_session_runs_end_to_end() {
    let mut session = make_session(100, 99);
    let result = session
        .sql("SELECT region, COUNT(*) AS n FROM events GROUP BY region ORDER BY n DESC LIMIT 3")
        .expect("sql should succeed");
    assert!(result.len() <= 3);
    assert!(result.column_names.contains(&"region".to_string()));
}

#[test]
fn test_session_tracks_query_count() {
    let mut session = make_session(50, 1);
    session.sql("SELECT region FROM events LIMIT 1").unwrap();
    session.sql("SELECT region FROM events LIMIT 1").unwrap();
    assert_eq!(session.queries_executed, 2);
}
