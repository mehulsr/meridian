//! Physical plan nodes.
//!
//! Port of `meridian.query.plan`. PlanNode is an enum of all operator types.
//! Children are `Box`-wrapped for recursion.

use crate::core::enums::AggFunc;
use crate::query::ast::Expr;

/// Port of `SeqScan` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct SeqScan {
    pub table: String,
}

impl SeqScan {
    pub fn new(table: impl Into<String>) -> SeqScan {
        SeqScan { table: table.into() }
    }
}

/// Port of `IndexScan` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexScan {
    pub table: String,
    pub index_name: String,
    pub key: String, // Simplified: Python stores arbitrary object
}

impl IndexScan {
    pub fn new(table: impl Into<String>, index_name: impl Into<String>) -> IndexScan {
        IndexScan {
            table: table.into(),
            index_name: index_name.into(),
            key: String::new(),
        }
    }
}

/// Port of `Filter` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub predicate: Expr,
    pub child: Box<PlanNode>,
}

impl Filter {
    pub fn new(predicate: Expr, child: PlanNode) -> Filter {
        Filter { predicate, child: Box::new(child) }
    }
}

/// Port of `Project` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub columns: Vec<String>,
    pub child: Box<PlanNode>,
}

impl Project {
    pub fn new(columns: Vec<String>, child: PlanNode) -> Project {
        Project { columns, child: Box::new(child) }
    }
}

/// Port of `Aggregate` dataclass. `aggs` is list of `(name, func, optional_arg)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregate {
    pub group_keys: Vec<String>,
    pub aggs: Vec<(String, AggFunc, Option<String>)>,
    pub child: Box<PlanNode>,
}

impl Aggregate {
    pub fn new(group_keys: Vec<String>, aggs: Vec<(String, AggFunc, Option<String>)>, child: PlanNode) -> Aggregate {
        Aggregate { group_keys, aggs, child: Box::new(child) }
    }
}

/// Port of `Sort` dataclass. `keys` is list of `(column_name, descending)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sort {
    pub keys: Vec<(String, bool)>,
    pub child: Box<PlanNode>,
}

impl Sort {
    pub fn new(keys: Vec<(String, bool)>, child: PlanNode) -> Sort {
        Sort { keys, child: Box::new(child) }
    }
}

/// Port of `Limit` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub count: usize,
    pub child: Box<PlanNode>,
}

impl Limit {
    pub fn new(count: usize, child: PlanNode) -> Limit {
        Limit { count, child: Box::new(child) }
    }
}

/// Port of `HashJoin` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct HashJoin {
    pub left: Box<PlanNode>,
    pub right: Box<PlanNode>,
    pub left_key: String,
    pub right_key: String,
}

impl HashJoin {
    pub fn new(left: PlanNode, right: PlanNode, left_key: impl Into<String>, right_key: impl Into<String>) -> HashJoin {
        HashJoin { left: Box::new(left), right: Box::new(right), left_key: left_key.into(), right_key: right_key.into() }
    }
}

/// Port of Python's `PlanNode = SeqScan | IndexScan | Filter | Project | Aggregate | Sort | Limit | HashJoin`.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanNode {
    SeqScan(SeqScan),
    IndexScan(IndexScan),
    Filter(Filter),
    Project(Project),
    Aggregate(Aggregate),
    Sort(Sort),
    Limit(Limit),
    HashJoin(HashJoin),
}

/// Port of `PhysicalPlan` dataclass.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalPlan {
    pub root: PlanNode,
    pub output_names: Vec<String>,
}

impl PhysicalPlan {
    pub fn new(root: PlanNode) -> PhysicalPlan {
        PhysicalPlan { root, output_names: Vec::new() }
    }

    pub fn with_output(root: PlanNode, output_names: Vec<String>) -> PhysicalPlan {
        PhysicalPlan { root, output_names }
    }
}
