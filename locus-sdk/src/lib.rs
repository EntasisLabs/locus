//! High-level Rust SDK for STTP memory workflows.
//!
//! `locus-sdk` composes low-level storage and parsing capabilities from `locus-core-rs`
//! into ergonomic workflow services for applications, agents, and operators.
//! The crate is transport-neutral and can run in-process or behind gateway surfaces.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;
pub mod prelude;
pub mod testing;
