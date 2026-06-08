//! Structured error hierarchy.
//!
//! Port of `meridian.core.errors`. Python uses an exception class hierarchy
//! (`MeridianError` base + one subclass per failure domain). In Rust we collapse
//! the hierarchy into a single `thiserror` enum with one variant per Python
//! subclass; each carries a message `String` (Python passed a `str` to the
//! exception constructor).

use thiserror::Error;

/// Port of the `MeridianError` hierarchy. One variant per Python subclass.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MeridianError {
    /// Python `SchemaError`.
    #[error("{0}")]
    Schema(String),
    /// Python `ParseError`.
    #[error("{0}")]
    Parse(String),
    /// Python `PlanError`.
    #[error("{0}")]
    Plan(String),
    /// Python `ExecutionError`.
    #[error("{0}")]
    Execution(String),
    /// Python `StorageError`.
    #[error("{0}")]
    Storage(String),
    /// Python `IngestError`.
    #[error("{0}")]
    Ingest(String),
    /// Bare `MeridianError` raised directly (rare; base class).
    #[error("{0}")]
    Other(String),
}

/// Convenience alias mirroring the pervasive `raise MeridianError` pattern.
pub type Result<T> = std::result::Result<T, MeridianError>;
