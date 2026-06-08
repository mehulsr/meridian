//! Meridian — columnar in-memory analytics engine.
//!
//! Rust port of the Python `meridian` package. The module tree mirrors
//! `src/meridian/` one-to-one so each Python module maps to a Rust module.
//!
//! Layering (bottom-up, no cycles):
//!   core -> util / storage -> index / query -> agg -> runtime / cli / bench
//!
//! All modules are currently stubs with empty implementations.

// Silence unused-code warnings while the port is still scaffolding.
#![allow(dead_code, unused_variables, unused_imports)]

pub mod core;
pub mod util;
pub mod storage;
pub mod index;
pub mod query;
pub mod agg;
pub mod ingest;
pub mod runtime;
pub mod bench;
pub mod cli;
