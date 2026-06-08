//! Port of `tests/test_parser.py` (W7: query::parser).

use meridian::core::enums::AggFunc;
use meridian::query::ast::{AggExpr, Query, SelectQuery};
use meridian::query::parser::parse_query;

#[test]
fn test_parse_numeric_where() {
    let q = parse_query("SELECT region FROM events WHERE amount > 10")
        .expect("parse failed");
    match q {
        Query::Select(SelectQuery { where_clause, .. }) => {
            assert!(where_clause.is_some());
        }
        _ => panic!("expected SelectQuery"),
    }
}

#[test]
fn test_parse_group_by_agg() {
    let q = parse_query(
        "SELECT region, COUNT(*) AS n FROM events GROUP BY region ORDER BY n DESC LIMIT 5",
    )
    .expect("parse failed");
    match q {
        Query::Select(SelectQuery {
            columns,
            group_by,
            limit,
            ..
        }) => {
            assert_eq!(group_by.len(), 1);
            assert_eq!(limit, Some(5));
            assert!(matches!(
                &columns[1].expr,
                meridian::query::ast::Expr::Agg(AggExpr {
                    func: AggFunc::Count,
                    ..
                })
            ));
        }
        _ => panic!("expected SelectQuery"),
    }
}
