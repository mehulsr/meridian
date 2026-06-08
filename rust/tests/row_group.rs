//! Port of `tests/test_row_group.py` (W3: storage::row_group).

use meridian::core::types::{DataType, Value};
use meridian::storage::chunk::encode_column_chunk;
use meridian::storage::row_group::{RowGroup, RowGroupCollection};

#[test]
fn test_row_group_zone_pruning() {
    let values: Vec<Value> = (0..100).map(Value::from_int).collect();
    let chunk = encode_column_chunk("x", DataType::Int64, &values, "auto");
    let mut group = RowGroup::new(0, 100);
    group.add_chunk("x", chunk);
    let collection = RowGroupCollection::new(vec![group]);
    let low = Value::from_int(200);
    let result = collection.scan("x", Some(&low), None);
    assert!(result.is_empty());
}
