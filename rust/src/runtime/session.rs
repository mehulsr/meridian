//! Port of meridian.runtime.session — query session state and catalog lifecycle.

use crate::core::catalog::Catalog;
use crate::core::config::{default_config, EngineConfig};
use crate::core::errors::Result;
use crate::query::executor::Executor;
use crate::query::parser::parse_query;
use crate::query::planner::Planner;
use crate::query::result::ResultSet;
use crate::storage::buffer_pool::DecodeCache;

pub struct Session {
    pub catalog: Catalog,
    pub config: EngineConfig,
    pub decode_cache: DecodeCache,
    pub queries_executed: usize,
}

impl Session {
    pub fn new() -> Self {
        let config = default_config();
        Session {
            catalog: Catalog::new(),
            decode_cache: DecodeCache::new(128),
            config,
            queries_executed: 0,
        }
    }

    /// Parse, plan, and execute a SQL string. Returns the ResultSet.
    pub fn sql(&mut self, text: &str) -> Result<ResultSet> {
        let query = parse_query(text)?;
        let plan = Planner::new(self.catalog.clone()).plan(&query)?;
        let mut executor = Executor::new(self.catalog.clone());
        self.queries_executed += 1;
        executor.execute(&plan)
    }

    pub fn reset_cache(&mut self) {
        self.decode_cache = DecodeCache::new(self.decode_cache.capacity);
    }
}

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}
