//! Port of meridian.ingest.validators

use std::collections::HashMap;

use crate::core::errors::{MeridianError, Result};
use crate::core::schema::Schema;
use crate::core::types::Value;

pub struct BatchValidator {
    pub schema: Schema,
    pub strict: bool,
    pub errors: Vec<String>,
}

impl BatchValidator {
    pub fn new(schema: Schema, strict: bool) -> Self {
        BatchValidator { schema, strict, errors: Vec::new() }
    }

    pub fn validate_batch(&mut self, batch: &HashMap<String, Vec<Value>>) -> Result<bool> {
        self.errors.clear();
        let lengths: std::collections::HashSet<usize> = batch.values().map(|v| v.len()).collect();
        if lengths.len() != 1 {
            self.errors.push(format!("column length mismatch: {:?}", lengths));
            if self.strict {
                return Err(MeridianError::Ingest(self.errors[0].clone()));
            }
            return Ok(false);
        }
        for col in &self.schema.columns {
            if let Some(values) = batch.get(&col.name) {
                for (i, value) in values.iter().enumerate() {
                    if value.is_null() && !col.nullable {
                        self.errors.push(format!("NULL at row {} column {}", i, col.name));
                    } else if !value.is_null() && value.data_type() != col.dtype {
                        if self.strict {
                            self.errors.push(format!(
                                "type mismatch row {} column {}: {:?} != {:?}",
                                i,
                                col.name,
                                value.data_type(),
                                col.dtype
                            ));
                        }
                    }
                }
            } else {
                self.errors.push(format!("missing column {}", col.name));
            }
        }
        let ok = self.errors.is_empty();
        if !ok && self.strict {
            let msg = self.errors.iter().take(5).cloned().collect::<Vec<_>>().join("; ");
            return Err(MeridianError::Ingest(msg));
        }
        Ok(ok)
    }
}
