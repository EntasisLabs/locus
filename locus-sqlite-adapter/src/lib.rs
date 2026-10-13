//! SQLite persistence for Locus [`NodeStore`] and [`SemanticIndexStore`].
//!
//! The logical tables match the Surreal schema: `temporal_node`, `calibration`,
//! `sync_checkpoint`, and `semantic_tag_index`. Hosts open one [`SqliteDatabase`]
//! and share it between [`SqliteNodeStore`] and [`SqliteSemanticIndexStore`].

mod db;
mod node_store;
mod rank;
mod semantic_index;

pub use db::SqliteDatabase;
pub use node_store::SqliteNodeStore;
pub use semantic_index::SqliteSemanticIndexStore;

#[cfg(test)]
mod tests;
