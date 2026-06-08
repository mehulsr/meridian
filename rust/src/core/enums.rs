//! Shared enumerations to avoid import cycles.
//!
//! Port of `meridian.core.enums`. Python uses `enum.Enum` with `auto()`; these
//! are pure tag enums (no associated data), so they map to `Copy` Rust enums.

/// Port of Python `AggFunc(Enum)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AggFunc {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Port of Python `CompareOp(Enum)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
