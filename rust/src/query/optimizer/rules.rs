//! Plan rewrite rules.
//!
//! Port of `meridian.query.optimizer.rules`. **Unwired** — minimal parity port.
//! Applies simple rewrite rules like predicate pushdown (column-to-literal only).

use crate::core::enums::CompareOp;
use crate::query::ast::{BinaryExpr, ColumnRef, Expr, LiteralExpr};
use crate::query::plan::{Filter, Limit, PlanNode, Project, SeqScan, Sort};

/// Port of `RuleEngine` class.
pub struct RuleEngine;

impl RuleEngine {
    /// Apply rewrite rules recursively to a plan node.
    pub fn apply(&self, node: PlanNode) -> PlanNode {
        match node {
            PlanNode::Filter(Filter { predicate, child }) => {
                let pushed = self.push_predicates(&predicate, &child);
                PlanNode::Filter(Filter {
                    predicate: pushed,
                    child: Box::new(self.apply(*child)),
                })
            }
            PlanNode::Project(Project { columns, child }) => {
                PlanNode::Project(Project {
                    columns,
                    child: Box::new(self.apply(*child)),
                })
            }
            PlanNode::Sort(Sort { keys, child }) => {
                // If child is a Limit, keep sort on top for now (don't reorder)
                PlanNode::Sort(Sort {
                    keys,
                    child: Box::new(self.apply(*child)),
                })
            }
            PlanNode::Limit(Limit { count, child }) => {
                PlanNode::Limit(Limit {
                    count,
                    child: Box::new(self.apply(*child)),
                })
            }
            other => other,
        }
    }

    /// Push predicates down through the tree (simplified: only handle column-to-literal).
    fn push_predicates(&self, predicate: &Expr, child: &PlanNode) -> Expr {
        if let PlanNode::SeqScan(_) = child {
            if let Expr::Binary(BinaryExpr { op, left, right }) = predicate {
                if let (Expr::Column(ColumnRef { name, .. }), Expr::Literal(_)) =
                    (left.as_ref(), right.as_ref())
                {
                    return predicate.clone();
                }
                if let (Expr::Literal(_), Expr::Column(ColumnRef { name, .. })) =
                    (left.as_ref(), right.as_ref())
                {
                    // Flip the operator: `literal op column` → `column flip(op) literal`
                    return Expr::Binary(BinaryExpr {
                        op: flip_op(*op),
                        left: right.clone(),
                        right: left.clone(),
                    });
                }
            }
        }
        predicate.clone()
    }
}

/// Flip a comparison operator (e.g., < ↔ >).
fn flip_op(op: CompareOp) -> CompareOp {
    match op {
        CompareOp::Lt => CompareOp::Gt,
        CompareOp::Le => CompareOp::Ge,
        CompareOp::Gt => CompareOp::Lt,
        CompareOp::Ge => CompareOp::Le,
        other => other,
    }
}
