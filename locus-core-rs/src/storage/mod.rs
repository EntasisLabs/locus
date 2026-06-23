//! Storage backends for STTP nodes.
//!
//! Includes in-memory storage for local/testing use and SurrealDB-backed
//! persistence for production-like deployments.

pub mod in_memory_node_store;
pub mod in_memory_semantic_index_store;
pub mod surrealdb;
pub mod tenant;

pub use in_memory_node_store::InMemoryNodeStore;
pub use in_memory_semantic_index_store::InMemorySemanticIndexStore;
pub use tenant::derive_tenant_id_from_session;
pub use surrealdb::{
    QueryParams, SurrealDbClient, SurrealDbEndpointsSettings, SurrealDbNodeStore,
    SurrealDbRuntimeOptions, SurrealDbSemanticIndexStore, SurrealDbSettings,
};
