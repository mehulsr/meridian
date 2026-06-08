//! Port of `tests/test_indexes.py` (W5: index::hash_index, index::bloom).

use meridian::core::types::{DataType, Value};
use meridian::index::bloom::BloomFilter;
use meridian::index::hash_index::HashIndex;
use meridian::storage::column::Column;

#[test]
fn test_hash_index_lookup() {
    let mut col = Column::new("id", DataType::Int64);
    col.append_many(vec![
        Value::from_int(1),
        Value::from_int(2),
        Value::from_int(2),
        Value::from_int(3),
    ]);
    let mut idx = HashIndex::new("id");
    idx.build(&col);
    assert_eq!(idx.lookup(&Value::from_int(2)), vec![1, 2]);
    assert!(idx.lookup(&Value::from_int(99)).is_empty());
}

#[test]
fn test_bloom_maybe_contains() {
    let mut col = Column::new("region", DataType::String);
    col.append_many(vec![Value::from_str("us-east"), Value::from_str("eu-central")]);
    let mut bloom = BloomFilter::new(10, 0.01);
    bloom.build(&col);
    assert!(bloom.maybe_contains(&Value::from_str("us-east")));
    assert!(!bloom.maybe_contains(&Value::from_str("missing-region-xyz")));
}
