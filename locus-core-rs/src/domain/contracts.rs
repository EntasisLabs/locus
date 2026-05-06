use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::models::{
    AvecState, BatchRekeyResult, ChangeQueryResult, ConnectorMetadata, NodeQuery, NodeUpsertResult,
    SttpNode, SyncCheckpoint, SyncCursor, ValidationResult,
};

/// Storage abstraction for STTP nodes and calibration data.
///
/// Implementors are expected to preserve semantics across both in-memory and
/// persistent backends.
#[async_trait]
pub trait NodeStore: Send + Sync {
    /// Query nodes with optional session and time filters.
    async fn query_nodes_async(&self, query: NodeQuery) -> Result<Vec<SttpNode>>;

    /// Persist a parsed node and return its storage identifier.
    async fn store_async(&self, node: SttpNode) -> Result<String> {
        Ok(self.upsert_node_async(node).await?.node_id)
    }

    /// Idempotently persist a parsed node using its deterministic sync key.
    async fn upsert_node_async(&self, node: SttpNode) -> Result<NodeUpsertResult>;

    /// Retrieve nodes ordered by resonance to the provided AVEC state.
    async fn get_by_resonance_async(
        &self,
        session_id: &str,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>>;

    /// Retrieve nodes ordered by resonance across all sessions.
    async fn get_by_resonance_global_async(
        &self,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>>;

    /// Retrieve nodes using blended AVEC resonance and semantic similarity.
    ///
    /// This is additive and backward-compatible with resonance-only callers.
    /// Implementations should gracefully fall back to AVEC-only ranking when
    /// embeddings are unavailable.
    async fn get_by_hybrid_async(
        &self,
        session_id: &str,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let _ = (query_embedding, alpha, beta);
        self.get_by_resonance_async(session_id, current_avec, from_utc, to_utc, tiers, limit)
            .await
    }

    /// Retrieve nodes using blended AVEC resonance and semantic similarity across all sessions.
    async fn get_by_hybrid_global_async(
        &self,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        query_embedding: Option<&[f32]>,
        alpha: f32,
        beta: f32,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let _ = (query_embedding, alpha, beta);
        self.get_by_resonance_global_async(current_avec, from_utc, to_utc, tiers, limit)
            .await
    }

    /// List recent nodes with an optional session filter.
    async fn list_nodes_async(
        &self,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<SttpNode>>;

    /// Read the most recent calibration AVEC for a session.
    async fn get_last_avec_async(&self, session_id: &str) -> Result<Option<AvecState>>;

    /// Read calibration trigger history for a session.
    async fn get_trigger_history_async(&self, session_id: &str) -> Result<Vec<String>>;

    /// Store a new calibration measurement for a session.
    async fn store_calibration_async(
        &self,
        session_id: &str,
        avec: AvecState,
        trigger: &str,
    ) -> Result<()>;

    /// Query nodes that changed after the provided cursor.
    async fn query_changes_since_async(
        &self,
        session_id: &str,
        cursor: Option<SyncCursor>,
        limit: usize,
    ) -> Result<ChangeQueryResult>;

    /// Read the last sync checkpoint for a connector within a session scope.
    async fn get_checkpoint_async(
        &self,
        session_id: &str,
        connector_id: &str,
    ) -> Result<Option<SyncCheckpoint>>;

    /// Persist the last sync checkpoint for a connector within a session scope.
    async fn put_checkpoint_async(&self, checkpoint: SyncCheckpoint) -> Result<()>;

    /// Batch-rekey one or more source scopes to a target scope using node IDs as anchors.
    ///
    /// Implementations should treat `node_ids` as source-scope anchors and apply scope-wide
    /// updates across all related tables, not just the anchor records themselves.
    async fn batch_rekey_scopes_async(
        &self,
        node_ids: Vec<String>,
        target_tenant_id: &str,
        target_session_id: &str,
        dry_run: bool,
        allow_merge: bool,
    ) -> Result<BatchRekeyResult>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn model_name(&self) -> &str;
    async fn embed_async(&self, text: &str) -> Result<Vec<f32>>;
}

/// One-time initializer contract for a storage backend.
///
/// This is typically used for schema creation and migration/backfill hooks.
#[async_trait]
pub trait NodeStoreInitializer: Send + Sync {
    async fn initialize_async(&self) -> Result<()>;
}

/// Validator contract for raw STTP node payloads.
pub trait NodeValidator: Send + Sync {
    /// Validate structural and semantic correctness of raw STTP text.
    fn validate(&self, raw_node: &str) -> ValidationResult;
    /// Verify PSI coherence between fields and computed values.
    fn verify_psi(&self, node: &SttpNode) -> bool;
}

#[async_trait]
pub trait SyncChangeSource: Send + Sync {
    async fn read_changes_async(
        &self,
        session_id: &str,
        connector_id: &str,
        cursor: Option<SyncCursor>,
        limit: usize,
    ) -> Result<ChangeQueryResult>;
}

pub trait SyncCoordinatorPolicy: Send + Sync {
    fn should_accept_node(&self, _node: &SttpNode) -> bool {
        true
    }

    fn checkpoint_metadata(
        &self,
        _session_id: &str,
        _connector_id: &str,
        previous: Option<&SyncCheckpoint>,
        _last_applied_node: Option<&SttpNode>,
        _next_cursor: Option<&SyncCursor>,
    ) -> Option<ConnectorMetadata> {
        previous.and_then(|checkpoint| checkpoint.metadata.clone())
    }
}
