//! Human-readable plan formatting.
//!
//! Port of `meridian.query.explain`. Recursively formats a `PlanNode` tree
//! as indented text (2 spaces per level).

use crate::query::plan::{Aggregate, Filter, HashJoin, Limit, PlanNode, Project, SeqScan, Sort};

/// Port of Python `explain_node` — recursively format a plan tree.
pub fn explain_node(node: &PlanNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match node {
        PlanNode::SeqScan(SeqScan { table }) => {
            format!("{}SeqScan(table={})", pad, table)
        }
        PlanNode::Filter(Filter { predicate: _, child }) => {
            let body = explain_node(child, indent + 1);
            format!("{}Filter\n{}", pad, body)
        }
        PlanNode::Project(Project { columns, child }) => {
            let body = explain_node(child, indent + 1);
            format!("{}Project(cols={:?})\n{}", pad, columns, body)
        }
        PlanNode::Aggregate(Aggregate { group_keys, aggs, child }) => {
            let body = explain_node(child, indent + 1);
            format!("{}Aggregate(keys={:?}, aggs={})\n{}", pad, group_keys, aggs.len(), body)
        }
        PlanNode::Sort(Sort { keys, child }) => {
            let body = explain_node(child, indent + 1);
            format!("{}Sort(keys={:?})\n{}", pad, keys, body)
        }
        PlanNode::Limit(Limit { count, child }) => {
            let body = explain_node(child, indent + 1);
            format!("{}Limit({})\n{}", pad, count, body)
        }
        PlanNode::HashJoin(HashJoin { left, right, left_key, right_key }) => {
            let left_str = explain_node(left, indent + 1);
            let right_str = explain_node(right, indent + 1);
            format!(
                "{}HashJoin({}={})\n{}  Left:\n{}\n{}  Right:\n{}",
                pad, left_key, right_key, pad, left_str, pad, right_str
            )
        }
        PlanNode::IndexScan(idx) => {
            format!("{}IndexScan(table={}, index={})", pad, idx.table, idx.index_name)
        }
    }
}
