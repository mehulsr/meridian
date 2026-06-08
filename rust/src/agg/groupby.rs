//! Port of meridian.agg.groupby — primary CPU hotspot.

use std::collections::HashMap;

use crate::agg::functions::apply_agg;
use crate::core::arena::StringInterner;
use crate::core::enums::AggFunc;
use crate::core::types::Value;
use crate::query::result::ResultSet;
use crate::storage::column::Column;
use crate::util::hash::hash_row_key;

pub struct GroupByEngine {
    interner: StringInterner,
}

impl GroupByEngine {
    pub fn new() -> Self {
        GroupByEngine { interner: StringInterner::new() }
    }

    /// Aggregate rows by key columns, emitting one result row per group.
    ///
    /// The Python implementation uses a chained-dict hash table keyed on a 24-bit
    /// bucket mask. We collapse the linked-list collision chain into a `Vec` of
    /// `(key, rows)` entries per bucket, preserving the `& 0xFFFFFF` mask and the
    /// `sorted(groups.keys())` output ordering.
    pub fn aggregate(
        &mut self,
        keys: &[Column],
        values: &[(String, AggFunc, Option<Column>)],
    ) -> ResultSet {
        let n = if !keys.is_empty() {
            keys[0].len()
        } else {
            values.first().and_then(|(_, _, c)| c.as_ref()).map(|c| c.len()).unwrap_or(0)
        };

        // bucket -> Vec<(key_tuple, row_ids)>
        let mut groups: HashMap<u64, Vec<(Vec<Value>, Vec<usize>)>> = HashMap::new();

        for row_id in 0..n {
            let key_tuple: Vec<Value> = keys
                .iter()
                .map(|k| self.normalize_key(k.get(row_id)))
                .collect();
            let bucket = hash_row_key(&key_tuple) & 0xFFFFFF;
            let chain = groups.entry(bucket).or_default();
            if let Some(entry) = chain.iter_mut().find(|(k, _)| k == &key_tuple) {
                entry.1.push(row_id);
            } else {
                chain.push((key_tuple, vec![row_id]));
            }
        }

        let column_names: Vec<String> = keys
            .iter()
            .map(|k| k.name.clone())
            .chain(values.iter().map(|(alias, _, _)| alias.clone()))
            .collect();

        let mut result = ResultSet::new(column_names);

        // Emit in sorted bucket order (matches Python `sorted(groups.keys())`)
        let mut buckets: Vec<u64> = groups.keys().copied().collect();
        buckets.sort_unstable();

        for bucket in buckets {
            let chain = &groups[&bucket];
            // Python's linked list prepends (newest-first); replicate by reversing.
            for (key_tuple, row_ids) in chain.iter().rev() {
                let mut out_row: Vec<Value> = key_tuple.clone();
                for (_, func, col) in values {
                    out_row.push(apply_agg(*func, col.as_ref(), Some(row_ids)));
                }
                result.add_row(out_row);
            }
        }

        result
    }

    fn normalize_key(&mut self, value: Value) -> Value {
        if let Value::Str(ref s) = value {
            let interned = self.interner.intern(s);
            return Value::Str(interned);
        }
        value
    }
}

impl Default for GroupByEngine {
    fn default() -> Self {
        GroupByEngine::new()
    }
}
