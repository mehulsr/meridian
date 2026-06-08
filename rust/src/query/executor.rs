//! Port of meridian.query.executor — Volcano-style iterator executor.

use std::collections::HashMap;

use crate::agg::groupby::GroupByEngine;
use crate::core::catalog::Catalog;
use crate::core::errors::{MeridianError, Result};
use crate::core::types::{DataType, Value};
use crate::query::operators::filter::filter_rows;
use crate::query::operators::join::hash_join;
use crate::query::operators::project::project_columns;
use crate::query::operators::scan::scan_table;
use crate::query::operators::sort::sort_rows;
use crate::query::plan::{Aggregate, Filter, HashJoin, Limit, PhysicalPlan, PlanNode, Project, SeqScan, Sort};
use crate::query::result::ResultSet;
use crate::storage::column::Column;

pub struct Executor {
    catalog: Catalog,
    group_by: GroupByEngine,
}

impl Executor {
    pub fn new(catalog: Catalog) -> Self {
        Executor { catalog, group_by: GroupByEngine::new() }
    }

    pub fn execute(&mut self, plan: &PhysicalPlan) -> Result<ResultSet> {
        let (columns, row_ids, mut names) = self.exec_node(&plan.root)?;
        if !plan.output_names.is_empty() {
            names = plan.output_names.clone();
        }
        let mut result = ResultSet::new(names.clone());
        for rid in &row_ids {
            let mut row = Vec::new();
            for name in &names {
                if let Some(col) = columns.get(name) {
                    row.push(col.get(*rid));
                }
            }
            result.add_row(row);
        }
        Ok(result)
    }

    fn exec_node(&mut self, node: &PlanNode) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        match node {
            PlanNode::SeqScan(seq_scan) => self.exec_seqscan(seq_scan),
            PlanNode::Filter(filter) => self.exec_filter(filter),
            PlanNode::Project(project) => self.exec_project(project),
            PlanNode::Aggregate(agg) => self.exec_aggregate(agg),
            PlanNode::Sort(sort) => self.exec_sort(sort),
            PlanNode::Limit(limit) => self.exec_limit(limit),
            PlanNode::HashJoin(join) => self.exec_hashjoin(join),
            _ => Err(MeridianError::Parse("unsupported plan node".into())),
        }
    }

    fn exec_seqscan(&self, node: &SeqScan) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let cols = scan_table(&self.catalog, &node.table)?;
        let n = cols.values().next().map(|c| c.len()).unwrap_or(0);
        let row_ids: Vec<usize> = (0..n).collect();
        let names = cols.keys().cloned().collect();
        Ok((cols, row_ids, names))
    }

    fn exec_filter(&mut self, node: &Filter) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let (cols, row_ids, names) = self.exec_node(&node.child)?;
        let filtered = filter_rows(&cols, &node.predicate);
        Ok((cols, filtered, names))
    }

    fn exec_project(&mut self, node: &Project) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let (cols, row_ids, _names) = self.exec_node(&node.child)?;
        let projected = project_columns(&cols, &node.columns, &row_ids);
        let new_cols = lists_to_columns(projected);
        let new_ids: Vec<usize> = (0..row_ids.len()).collect();
        let out_names = if node.columns == vec!["*"] { cols.keys().cloned().collect() } else { node.columns.clone() };
        Ok((new_cols, new_ids, out_names))
    }

    fn exec_aggregate(&mut self, node: &Aggregate) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let (cols, _row_ids, _names) = self.exec_node(&node.child)?;
        let key_cols: Vec<Column> = node.group_keys.iter().filter_map(|k| cols.get(k).cloned()).collect();
        let value_specs: Vec<(String, crate::core::enums::AggFunc, Option<Column>)> = node
            .aggs
            .iter()
            .map(|(alias, func, arg)| (alias.clone(), *func, arg.as_ref().and_then(|a| cols.get(a).cloned())))
            .collect();
        let rs = self.group_by.aggregate(&key_cols, &value_specs);
        let out_cols = resultset_to_columns(&rs);
        let out_ids: Vec<usize> = (0..rs.len()).collect();
        let names = rs.column_names.clone();
        Ok((out_cols, out_ids, names))
    }

    fn exec_sort(&mut self, node: &Sort) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let (cols, row_ids, names) = self.exec_node(&node.child)?;
        let sorted_ids = sort_rows(&cols, &row_ids, &node.keys);
        Ok((cols, sorted_ids, names))
    }

    fn exec_limit(&mut self, node: &Limit) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let (cols, mut row_ids, names) = self.exec_node(&node.child)?;
        row_ids.truncate(node.count);
        Ok((cols, row_ids, names))
    }

    fn exec_hashjoin(&mut self, node: &HashJoin) -> Result<(HashMap<String, Column>, Vec<usize>, Vec<String>)> {
        let (lcols, _lids, _lnames) = self.exec_node(&node.left)?;
        let (rcols, _rids, _rnames) = self.exec_node(&node.right)?;
        let (merged, mids, _) = hash_join(&lcols, &rcols, &node.left_key, &node.right_key);
        let names = merged.keys().cloned().collect();
        Ok((merged, mids, names))
    }
}

fn lists_to_columns(data: HashMap<String, Vec<Value>>) -> HashMap<String, Column> {
    let mut out = HashMap::new();
    for (name, values) in data {
        let dtype = values.first().map(|v| v.data_type()).unwrap_or(DataType::String);
        let mut col = Column::new(name, dtype);
        col.append_many(values);
        out.insert(col.name.clone(), col);
    }
    out
}

fn resultset_to_columns(rs: &ResultSet) -> HashMap<String, Column> {
    let mut out = HashMap::new();
    if rs.rows.is_empty() {
        return out;
    }
    for (idx, name) in rs.column_names.iter().enumerate() {
        let dtype = rs.rows[0][idx].data_type();
        let mut col = Column::new(name.clone(), dtype);
        let values: Vec<Value> = rs.rows.iter().map(|row| row[idx].clone()).collect();
        col.append_many(values);
        out.insert(name.clone(), col);
    }
    out
}
