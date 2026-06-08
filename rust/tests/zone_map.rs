//! Port of `tests/test_zone_map.py` (W5: index::zone_map).

use meridian::core::types::{DataType, Value};
use meridian::index::zone_map::ZoneMap;
use meridian::storage::chunk::encode_column_chunk;

#[test]
fn test_zone_map_prunes_chunks() {
    let values: Vec<Value> = (0..1000).map(Value::from_int).collect();
    let mut chunks = Vec::new();
    for i in (0..1000).step_by(100) {
        chunks.push(encode_column_chunk("x", DataType::Int64, &values[i..i + 100], "auto"));
    }
    let mut zm = ZoneMap::new("x");
    zm.build_from_chunks(&chunks);
    let candidates = zm.candidate_chunks(Some(&Value::from_int(250)), Some(&Value::from_int(350)));
    assert!(candidates.iter().all(|&c| c >= 1 && c <= 4));
}
