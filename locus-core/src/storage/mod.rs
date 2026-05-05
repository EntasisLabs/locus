//! Storage backends for STTP nodes.
//!
//! Includes in-memory storage for local/testing use and SurrealDB-backed
//! persistence for production-like deployments.

pub mod in_memory_node_store;
pub mod surrealdb;

pub use in_memory_node_store::InMemoryNodeStore;
pub use surrealdb::{
    QueryParams, SurrealDbClient, SurrealDbEndpointsSettings, SurrealDbNodeStore,
    SurrealDbRuntimeOptions, SurrealDbSettings,
};
