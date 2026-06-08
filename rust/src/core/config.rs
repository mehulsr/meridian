//! Engine-wide configuration.
//!
//! Port of `meridian.core.config`. The Python `@dataclass EngineConfig` maps to
//! a `#[derive(Clone)]` struct with a `Default` impl carrying the same defaults.
//! `with_overrides(**kwargs)` is a partial update; in Rust that's idiomatically
//! `EngineConfig { field: new, ..base }`, so no kwargs shim is needed. The
//! `extra: dict` escape hatch maps to a `HashMap<String, String>`.

use std::collections::HashMap;

/// Port of Python `EngineConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub chunk_size: usize,
    pub default_batch_size: usize,
    pub hash_index_buckets: usize,
    pub enable_bloom_prefilter: bool,
    pub enable_zone_maps: bool,
    pub max_rows_in_memory: usize,
    pub query_timeout_ms: u64,
    pub profile_operators: bool,
    pub cache_chunk_decodes: bool,
    /// Python free-form `str`: "hash" | "nested".
    pub join_strategy: String,
    /// Python `extra: dict` escape hatch.
    pub extra: HashMap<String, String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            chunk_size: 8192,
            default_batch_size: 5000,
            hash_index_buckets: 65536,
            enable_bloom_prefilter: true,
            enable_zone_maps: true,
            max_rows_in_memory: 10_000_000,
            query_timeout_ms: 30_000,
            profile_operators: false,
            cache_chunk_decodes: false,
            join_strategy: "hash".to_string(),
            extra: HashMap::new(),
        }
    }
}

impl EngineConfig {
    /// Python `EngineConfig()` — the default constructor.
    pub fn new() -> Self {
        EngineConfig::default()
    }
}

/// Port of Python module-level `DEFAULT_CONFIG = EngineConfig()`.
///
/// Python binds a single shared instance at import; Rust has no non-const global
/// without `lazy_static`/`OnceLock`, and the value is cheap to build, so callers
/// use `EngineConfig::default()`. Provided as a function for call-site parity.
pub fn default_config() -> EngineConfig {
    EngineConfig::default()
}
