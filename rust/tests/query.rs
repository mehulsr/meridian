use meridian::core::catalog::Catalog;
use meridian::core::enums::AggFunc;
use meridian::core::types::{DataType, Value};
use meridian::query::ast::{ColumnRef, Expr, Query, SelectItem, SelectQuery};
use meridian::query::executor::Executor;
use meridian::query::planner::Planner;
use meridian::storage::column::Column;
use meridian::storage::table::Table;

#[test]
fn test_planner_creates_seqscan() {
    let catalog = Catalog::new();
    let planner = Planner::new(catalog);

    let query = Query::Select(SelectQuery {
        columns: vec![SelectItem::new(Expr::Column(ColumnRef::new("id")))],
        from_table: "test".into(),
        where_clause: None,
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
    });

    let plan = planner.plan(&query);
    assert!(plan.is_ok());
}

#[test]
fn test_executor_executes_with_empty_table() {
    let mut catalog = Catalog::new();
    let table = Table::new("test", meridian::core::schema::Schema::new());
    let _ = catalog.register_table(table);

    let catalog2 = catalog.clone();
    let mut executor = Executor::new(catalog);
    let query = Query::Select(SelectQuery {
        columns: vec![],
        from_table: "test".into(),
        where_clause: None,
        group_by: vec![],
        having: None,
        order_by: vec![],
        limit: None,
    });

    let planner = Planner::new(catalog2);
    let plan = planner.plan(&query).expect("plan should succeed");
    let result = executor.execute(&plan);
    assert!(result.is_ok());
}
