//! Query AST nodes.
//!
//! Port of `meridian.query.ast`. Python uses dataclass unions; here enums
//! with `Box`-wrapped recursive children (Expr, Query).

use crate::core::enums::{AggFunc, CompareOp};
use crate::core::types::Value;

/// Port of `ColumnRef` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnRef {
    pub name: String,
    pub alias: Option<String>,
}

impl ColumnRef {
    pub fn new(name: impl Into<String>) -> ColumnRef {
        ColumnRef { name: name.into(), alias: None }
    }

    pub fn with_alias(name: impl Into<String>, alias: impl Into<String>) -> ColumnRef {
        ColumnRef { name: name.into(), alias: Some(alias.into()) }
    }
}

/// Port of `LiteralExpr` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralExpr {
    pub value: Value,
}

impl LiteralExpr {
    pub fn new(value: Value) -> LiteralExpr {
        LiteralExpr { value }
    }
}

/// Port of `BinaryExpr` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    pub op: CompareOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

impl BinaryExpr {
    pub fn new(op: CompareOp, left: Expr, right: Expr) -> BinaryExpr {
        BinaryExpr { op, left: Box::new(left), right: Box::new(right) }
    }
}

/// Port of `AndExpr` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct AndExpr {
    pub parts: Vec<Expr>,
}

impl AndExpr {
    pub fn new(parts: Vec<Expr>) -> AndExpr {
        AndExpr { parts }
    }
}

/// Port of `AggExpr` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct AggExpr {
    pub func: AggFunc,
    pub arg: Option<ColumnRef>,
}

impl AggExpr {
    pub fn new(func: AggFunc) -> AggExpr {
        AggExpr { func, arg: None }
    }

    pub fn with_arg(func: AggFunc, arg: ColumnRef) -> AggExpr {
        AggExpr { func, arg: Some(arg) }
    }
}

/// Port of Python's `Expr = ColumnRef | LiteralExpr | BinaryExpr | AndExpr | AggExpr`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Column(ColumnRef),
    Literal(LiteralExpr),
    Binary(BinaryExpr),
    And(AndExpr),
    Agg(AggExpr),
}

/// Port of `SelectItem` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<String>,
}

impl SelectItem {
    pub fn new(expr: Expr) -> SelectItem {
        SelectItem { expr, alias: None }
    }

    pub fn with_alias(expr: Expr, alias: impl Into<String>) -> SelectItem {
        SelectItem { expr, alias: Some(alias.into()) }
    }
}

/// Port of `OrderByItem` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByItem {
    pub expr: ColumnRef,
    pub descending: bool,
}

impl OrderByItem {
    pub fn new(expr: ColumnRef) -> OrderByItem {
        OrderByItem { expr, descending: false }
    }

    pub fn descending(expr: ColumnRef) -> OrderByItem {
        OrderByItem { expr, descending: true }
    }
}

/// Port of `SelectQuery` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectQuery {
    pub columns: Vec<SelectItem>,
    pub from_table: String,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<ColumnRef>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<usize>,
}

impl SelectQuery {
    pub fn new(columns: Vec<SelectItem>, from_table: impl Into<String>) -> SelectQuery {
        SelectQuery {
            columns,
            from_table: from_table.into(),
            where_clause: None,
            group_by: Vec::new(),
            having: None,
            order_by: Vec::new(),
            limit: None,
        }
    }
}

/// Port of `JoinSpec` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinSpec {
    pub table: String,
    pub left_column: String,
    pub right_column: String,
    pub join_type: String,
}

impl JoinSpec {
    pub fn new(
        table: impl Into<String>,
        left_column: impl Into<String>,
        right_column: impl Into<String>,
    ) -> JoinSpec {
        JoinSpec {
            table: table.into(),
            left_column: left_column.into(),
            right_column: right_column.into(),
            join_type: "inner".to_string(),
        }
    }
}

/// Port of `JoinQuery` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinQuery {
    pub left: SelectQuery,
    pub join: JoinSpec,
    pub right: SelectQuery,
}

/// Port of Python's `Query = SelectQuery | JoinQuery`.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Select(SelectQuery),
    Join(JoinQuery),
}
