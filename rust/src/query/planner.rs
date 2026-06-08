//! Port of meridian.query.planner — naive cost-based query planner.

use crate::core::catalog::Catalog;
use crate::core::enums::AggFunc;
use crate::core::errors::{MeridianError, Result};
use crate::query::ast::{AggExpr, ColumnRef, Expr, JoinQuery, Query, SelectItem, SelectQuery};
use crate::query::plan::{Aggregate, Filter, HashJoin, Limit, PhysicalPlan, PlanNode, Project, SeqScan, Sort};

pub struct Planner {
    pub catalog: Catalog,
}

impl Planner {
    pub fn new(catalog: Catalog) -> Self {
        Planner { catalog }
    }

    pub fn plan(&self, query: &Query) -> Result<PhysicalPlan> {
        match query {
            Query::Join(jq) => self.plan_join(jq),
            Query::Select(sq) => self.plan_select(sq),
        }
    }

    fn plan_select(&self, query: &SelectQuery) -> Result<PhysicalPlan> {
        let mut node: PlanNode = PlanNode::SeqScan(SeqScan::new(query.from_table.clone()));
        let output_names = self.output_names(&query.columns);

        if let Some(where_expr) = &query.where_clause {
            node = PlanNode::Filter(Filter::new(where_expr.clone(), node));
        }

        if !query.group_by.is_empty() {
            let aggs = self.extract_aggs(&query.columns);
            let keys = query.group_by.iter().map(|c| c.name.clone()).collect();
            node = PlanNode::Aggregate(Aggregate::new(keys, aggs, node));
        } else if query.columns.iter().any(|i| matches!(i.expr, Expr::Agg(_))) {
            let aggs = self.extract_aggs(&query.columns);
            node = PlanNode::Aggregate(Aggregate::new(Vec::new(), aggs, node));
        } else {
            let cols = self.project_columns(&query.columns);
            if cols != vec!["*"] {
                node = PlanNode::Project(Project::new(cols, node));
            }
        }

        if !query.order_by.is_empty() {
            let sort_keys = query
                .order_by
                .iter()
                .map(|o| (o.expr.name.clone(), o.descending))
                .collect();
            node = PlanNode::Sort(Sort::new(sort_keys, node));
        }

        if let Some(limit) = query.limit {
            node = PlanNode::Limit(Limit::new(limit, node));
        }

        Ok(PhysicalPlan::with_output(node, output_names))
    }

    fn plan_join(&self, query: &JoinQuery) -> Result<PhysicalPlan> {
        let left_plan = self.plan(&Query::Select(query.left.clone()))?;
        let right_plan = self.plan(&Query::Select(query.right.clone()))?;
        let root = PlanNode::HashJoin(HashJoin::new(
            left_plan.root,
            right_plan.root,
            &query.join.left_column,
            &query.join.right_column,
        ));
        let mut names = left_plan.output_names;
        names.extend(right_plan.output_names);
        Ok(PhysicalPlan::with_output(root, names))
    }

    fn extract_aggs(&self, items: &[SelectItem]) -> Vec<(String, AggFunc, Option<String>)> {
        let mut out = Vec::new();
        for item in items {
            if let Expr::Agg(agg_expr) = &item.expr {
                let alias = item.alias.clone().unwrap_or_else(|| self.default_agg_name(agg_expr));
                let arg = agg_expr.arg.as_ref().map(|a| a.name.clone());
                out.push((alias, agg_expr.func, arg));
            } else if let Expr::Column(col_ref) = &item.expr {
                let alias = item.alias.clone().unwrap_or_else(|| col_ref.name.clone());
                out.push((alias, AggFunc::Max, Some(col_ref.name.clone())));
            }
        }
        out
    }

    fn project_columns(&self, items: &[SelectItem]) -> Vec<String> {
        if items.len() == 1 {
            if let Expr::Column(ColumnRef { name, .. }) = &items[0].expr {
                if name == "*" {
                    return vec!["*".into()];
                }
            }
        }
        let mut cols = Vec::new();
        for item in items {
            if let Expr::Column(col_ref) = &item.expr {
                cols.push(col_ref.name.clone());
            }
        }
        cols
    }

    fn output_names(&self, items: &[SelectItem]) -> Vec<String> {
        let mut names = Vec::new();
        for item in items {
            if let Some(alias) = &item.alias {
                names.push(alias.clone());
            } else if let Expr::Column(col_ref) = &item.expr {
                names.push(col_ref.name.clone());
            } else if let Expr::Agg(agg_expr) = &item.expr {
                names.push(self.default_agg_name(agg_expr));
            }
        }
        names
    }

    fn default_agg_name(&self, expr: &AggExpr) -> String {
        let arg = expr.arg.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| "*".into());
        format!("{}_{}", format!("{:?}", expr.func).to_lowercase(), arg)
    }
}
