use meridian::agg::groupby::GroupByEngine;
use meridian::core::enums::AggFunc;
use meridian::core::types::{DataType, Value};
use meridian::storage::column::Column;

#[test]
fn test_groupby_count_sum() {
    let mut region = Column::new("region", DataType::String);
    let mut amount = Column::new("amount", DataType::Float64);
    region.append_many(vec![
        Value::Str("a".into()),
        Value::Str("a".into()),
        Value::Str("b".into()),
    ]);
    amount.append_many(vec![
        Value::Float64(10.0),
        Value::Float64(5.0),
        Value::Float64(7.0),
    ]);

    let mut engine = GroupByEngine::new();
    let rs = engine.aggregate(
        &[region],
        &[
            ("n".into(), AggFunc::Count, None),
            ("total".into(), AggFunc::Sum, Some(amount)),
        ],
    );

    assert_eq!(rs.len(), 2);

    // Order-insensitive: build a map by region string
    let mut by_region: std::collections::HashMap<String, (i64, f64)> =
        std::collections::HashMap::new();
    for row in &rs.rows {
        let region_str = match &row[0] {
            Value::Str(s) => s.clone(),
            _ => panic!("expected Str"),
        };
        let count = match row[1] {
            Value::Int64(n) => n,
            _ => panic!("expected Int64 for count"),
        };
        let total = match row[2] {
            Value::Float64(f) => f,
            _ => panic!("expected Float64 for sum"),
        };
        by_region.insert(region_str, (count, total));
    }

    assert_eq!(by_region["a"], (2, 15.0));
    assert_eq!(by_region["b"], (1, 7.0));
}
