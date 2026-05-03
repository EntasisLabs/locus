//! Core Rust library for working with STTP nodes, AVEC calibration, and context retrieval.
//!
//! This crate provides:
//! - domain models and contracts,
//! - parsers and validators for STTP node payloads,
//! - application services for calibration, storage, retrieval, mood presets, rollups, and rekey,
//! - storage implementations (in-memory and SurrealDB-backed).
//!
//! # Quick Start
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use locus_core::{
//!     CalibrationService, ContextQueryService, InMemoryNodeStore, NodeStoreInitializer,
//!     StoreContextService, TreeSitterValidator,
//! };
//!
//! # fn main() -> anyhow::Result<()> {
//! let runtime = tokio::runtime::Builder::new_current_thread()
//!     .enable_all()
//!     .build()?;
//!
//! runtime.block_on(async {
//!     let store = Arc::new(InMemoryNodeStore::new());
//!     let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
//!     initializer.initialize_async().await?;
//!
//!     let validator = Arc::new(TreeSitterValidator::new());
//!     let store_context = StoreContextService::new(store.clone(), validator);
//!     let calibration = CalibrationService::new(store.clone());
//!     let context_query = ContextQueryService::new(store);
//!
//!     let raw_node = r#"
//! ⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "demo", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "demo", relevant_tier: raw, retrieval_budget: 3 } } ⟩
//! ⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "demo", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
//! ◈⟨ { note(.99): "example" } ⟩
//! ⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
//! "#;
//!
//!     let store_result = store_context.store_async(raw_node, "demo-session").await;
//!     if store_result.valid {
//!         let _ = calibration
//!             .calibrate_async("demo-session", 0.9, 0.2, 0.9, 0.85, "manual")
//!             .await?;
//!         let _ = context_query.get_context_async("demo-session", 0.9, 0.2, 0.9, 0.85, 5).await;
//!     }
//!
//!     Ok::<(), anyhow::Error>(())
//! })?;
//! # Ok(())
//! # }
//! ```

pub mod application;
pub mod domain;
pub mod parsing;
pub mod storage;

pub use application::services::{
    CalibrationService, ContextQueryService, EmbeddingMigrationFilter, EmbeddingMigrationMode,
    EmbeddingMigrationPreviewRequest, EmbeddingMigrationPreviewResult,
    EmbeddingMigrationRunRequest, EmbeddingMigrationRunResult, EmbeddingMigrationSample,
    EmbeddingMigrationService, MonthlyRollupService, MoodCatalogService, RekeyScopeService,
    StoreContextService, SyncCoordinatorService,
};
pub use application::validation::TreeSitterValidator;
pub use domain::contracts::{
    NodeStore, NodeStoreInitializer, NodeValidator, SyncChangeSource, SyncCoordinatorPolicy,
};
pub use domain::models::*;
pub use parsing::SttpNodeParser;
pub use storage::{
    InMemoryNodeStore, QueryParams, SurrealDbClient, SurrealDbEndpointsSettings,
    SurrealDbNodeStore, SurrealDbRuntimeOptions, SurrealDbSettings,
};
