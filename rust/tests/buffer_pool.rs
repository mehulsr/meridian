//! Port of `tests/test_buffer_pool.py` (W3: storage::buffer_pool).

use meridian::core::types::{DataType, Value};
use meridian::storage::buffer_pool::DecodeCache;
use meridian::storage::chunk::encode_column_chunk;

#[test]
fn test_decode_cache_tracks_hits() {
    let values: Vec<Value> = (0..50).map(Value::from_int).collect();
    let chunk = encode_column_chunk("x", DataType::Int64, &values, "auto");
    let mut cache = DecodeCache::new(4);
    let decoded = chunk.decode_all();
    cache.put("t", &chunk, 0, decoded);
    assert!(cache.get("t", &chunk, 0).is_some());
    assert!(cache.get("t", &chunk, 1).is_none());
    let stats = cache.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
}
