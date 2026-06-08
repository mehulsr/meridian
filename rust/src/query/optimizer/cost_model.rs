//! Naive cost model for physical planning.
//!
//! Port of `meridian.query.optimizer.cost_model`. **Unwired** — minimal parity port.
//! Estimates plan cost (higher = more expensive) and orders tables for joins.

use std::collections::HashMap;

use crate::query::optimizer::stats::TableStats;
use crate::query::plan::{Aggregate, Filter, HashJoin, Limit, PlanNode, Project, SeqScan, Sort};

/// Port of `CostModel` class with hardcoded cost factors.
pub struct CostModel {
    pub seq_scan_cost: f64,
    pub filter_selectivity: f64,
    pub hash_build_cost: f64,
    pub hash_probe_cost: f64,
    pub sort_cost_factor: f64,
    pub agg_cost_factor: f64,
}

impl CostModel {
    pub fn new() -> CostModel {
        CostModel {
            seq_scan_cost: 1.0,
            filter_selectivity: 0.25,
            hash_build_cost: 1.5,
            hash_probe_cost: 1.2,
            sort_cost_factor: 1.8,
            agg_cost_factor: 2.0,
        }
    }

    /// Estimate the cost of a plan node.
    pub fn estimate(&self, node: &PlanNode, stats: &HashMap<String, TableStats>) -> f64 {
        match node {
            PlanNode::SeqScan(SeqScan { table }) => {
                let rows = stats
                    .get(table)
                    .map(|s| s.row_count as f64)
                    .unwrap_or(1000.0);
                rows * self.seq_scan_cost
            }
            PlanNode::Filter(Filter { child, .. }) => {
                self.estimate(child, stats) * (1.0 + self.filter_selectivity)
            }
            PlanNode::Project(Project { child, .. }) => {
                self.estimate(child, stats) * 1.1
            }
            PlanNode::Aggregate(Aggregate { child, .. }) => {
                self.estimate(child, stats) * self.agg_cost_factor
            }
            PlanNode::Sort(Sort { child, .. }) => {
                let base = self.estimate(child, stats);
                base * self.sort_cost_factor
            }
            PlanNode::Limit(Limit { child, .. }) => {
                self.estimate(child, stats) * 0.5
            }
            PlanNode::HashJoin(HashJoin { left, right, .. }) => {
                let left_cost = self.estimate(left, stats);
                let right_cost = self.estimate(right, stats);
                (left_cost + right_cost) * self.hash_build_cost
                    + left_cost.min(right_cost) * self.hash_probe_cost
            }
            PlanNode::IndexScan(_) => 1000.0,
        }
    }

    /// Order tables for join by estimated row count (smallest first).
    pub fn choose_join_order(
        &self,
        tables: &[String],
        stats: &HashMap<String, TableStats>,
    ) -> Vec<String> {
        let mut sorted = tables.to_vec();
        sorted.sort_by_key(|t| stats.get(t).map(|s| s.row_count).unwrap_or(0));
        sorted
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::new()
    }
}
