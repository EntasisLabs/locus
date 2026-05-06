use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::contracts::{NodeStore, NodeStoreInitializer};
use crate::domain::models::{
    AvecState, BatchRekeyResult, ChangeQueryResult, ConnectorMetadata, NodeQuery, NodeUpsertResult,
    NodeUpsertStatus, ScopeRekeyResult, SttpNode, SyncCheckpoint, SyncCursor,
};
use crate::storage::surrealdb::client::{QueryParams, SurrealDbClient};
use crate::storage::surrealdb::models::{
    SurrealAvecRecord, SurrealCheckpointRecord, SurrealExistingNodeRecord, SurrealNodeRecord,
    SurrealTriggerRecord,
};
use crate::storage::surrealdb::raw_queries;

const DEFAULT_TENANT: &str = "default";
const TENANT_SCOPE_PREFIX: &str = "tenant:";
const TENANT_SCOPE_SEPARATOR: &str = "::session:";

#[derive(Debug, Deserialize)]
struct MissingTenantRecord {
    #[serde(default)]
    id: Value,
    #[serde(default)]
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct LegacyTemporalRecord {
    #[serde(default)]
    id: Value,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    sync_key: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScopeAnchorRecord {
    #[serde(rename = "TenantId", default)]
    tenant_id: Option<String>,
    #[serde(rename = "SessionId", default)]
    session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ScopeCountRecord {
    #[serde(rename = "Count", default)]
    count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct ScopeKey {
    tenant_id: String,
    session_id: String,
}

pub struct SurrealDbNodeStore {
    client: Arc<dyn SurrealDbClient>,
}

impl SurrealDbNodeStore {
    pub fn new(client: Arc<dyn SurrealDbClient>) -> Self {
        Self { client }
    }

    async fn backfill_missing_tenant_ids_async(&self) -> Result<()> {
        self.backfill_temporal_node_legacy_fields_async().await?;

        self.backfill_table_tenant_ids_async(
            "calibration",
            raw_queries::SELECT_CALIBRATION_MISSING_TENANT_QUERY,
        )
        .await?;

        Ok(())
    }

    async fn backfill_temporal_node_legacy_fields_async(&self) -> Result<()> {
        let rows = self
            .client
            .raw_query(
                raw_queries::SELECT_TEMPORAL_NODE_LEGACY_SYNC_QUERY,
                QueryParams::new(),
            )
            .await?;
        let records: Vec<LegacyTemporalRecord> = decode_rows(rows)?;

        for record in records {
            let session_id = record.session_id;
            if session_id.trim().is_empty() {
                continue;
            }

            let Some(record_id) = normalize_record_id(record.id, "temporal_node")
                .and_then(|value| normalize_temporal_node_id(&value))
            else {
                continue;
            };

            let mut parameters = QueryParams::new();
            parameters.insert(
                "tenant_id".to_string(),
                json!(derive_tenant_id_from_session(&session_id)),
            );
            parameters.insert(
                "sync_key".to_string(),
                json!(normalize_legacy_sync_key(
                    record.sync_key.as_deref(),
                    &record_id
                )),
            );
            parameters.insert(
                "updated_at".to_string(),
                json!(resolve_legacy_updated_at(
                    record.updated_at.as_deref(),
                    record.timestamp.as_deref(),
                )),
            );

            let query = raw_queries::update_temporal_node_legacy_sync_query(&record_id);
            self.client.raw_query(&query, parameters).await?;
        }

        Ok(())
    }

    async fn backfill_table_tenant_ids_async(&self, table: &str, select_query: &str) -> Result<()> {
        let rows = self
            .client
            .raw_query(select_query, QueryParams::new())
            .await?;
        let records: Vec<MissingTenantRecord> = decode_rows(rows)?;

        for record in records {
            let session_id = record.session_id;
            if session_id.trim().is_empty() {
                continue;
            }

            let Some(record_id) = normalize_record_id(record.id, table) else {
                continue;
            };

            let mut parameters = QueryParams::new();
            parameters.insert(
                "tenant_id".to_string(),
                json!(derive_tenant_id_from_session(&session_id)),
            );

            let query = raw_queries::update_record_tenant_query(&record_id);
            self.client.raw_query(&query, parameters).await?;
        }

        Ok(())
    }

    async fn count_scope_rows_async(
        &self,
        query: &str,
        session_id: &str,
        tenant_id: &str,
        include_legacy: bool,
    ) -> Result<usize> {
        let mut parameters = QueryParams::new();
        parameters.insert("session_id".to_string(), json!(session_id));
        parameters.insert("tenant_id".to_string(), json!(tenant_id));
        parameters.insert("include_legacy".to_string(), json!(include_legacy));

        let rows = self.client.raw_query(query, parameters).await?;
        let counts: Vec<ScopeCountRecord> = decode_rows(rows)?;
        Ok(counts.first().map(|value| value.count).unwrap_or(0))
    }

    async fn apply_scope_rekey_async(
        &self,
        source_session_id: &str,
        source_tenant_id: &str,
        target_session_id: &str,
        target_tenant_id: &str,
    ) -> Result<()> {
        let mut parameters = QueryParams::new();
        parameters.insert("source_session_id".to_string(), json!(source_session_id));
        parameters.insert("source_tenant_id".to_string(), json!(source_tenant_id));
        parameters.insert(
            "source_include_legacy".to_string(),
            json!(includes_legacy_tenant_bucket(source_tenant_id)),
        );
        parameters.insert("target_session_id".to_string(), json!(target_session_id));
        parameters.insert("target_tenant_id".to_string(), json!(target_tenant_id));

        self.client
            .raw_query(raw_queries::APPLY_SCOPE_REKEY_QUERY, parameters)
            .await?;

        Ok(())
    }

    async fn find_existing_node_any_tenant_async(
        &self,
        session_id: &str,
        sync_key: &str,
    ) -> Result<Option<SurrealExistingNodeRecord>> {
        let mut lookup_parameters = QueryParams::new();
        lookup_parameters.insert("session_id".to_string(), json!(session_id));
        lookup_parameters.insert("sync_key".to_string(), json!(sync_key));

        let rows = self
            .client
            .raw_query(
                raw_queries::FIND_EXISTING_NODE_BY_SYNC_KEY_ANY_TENANT_QUERY,
                lookup_parameters,
            )
            .await?;
        let records: Vec<SurrealExistingNodeRecord> = decode_rows(rows)?;

        Ok(records.into_iter().next())
    }

    async fn find_existing_node_exact_async(
        &self,
        tenant_id: &str,
        session_id: &str,
        sync_key: &str,
    ) -> Result<Option<SurrealExistingNodeRecord>> {
        let mut lookup_parameters = QueryParams::new();
        lookup_parameters.insert("tenant_id".to_string(), json!(tenant_id));
        lookup_parameters.insert("session_id".to_string(), json!(session_id));
        lookup_parameters.insert("sync_key".to_string(), json!(sync_key));

        let rows = self
            .client
            .raw_query(
                raw_queries::FIND_EXISTING_NODE_BY_SYNC_KEY_EXACT_QUERY,
                lookup_parameters,
            )
            .await?;
        let records: Vec<SurrealExistingNodeRecord> = decode_rows(rows)?;

        Ok(records.into_iter().next())
    }

    async fn update_existing_temporal_node_async(
        &self,
        record_id: &str,
        include_parent_assignment: bool,
        include_source_metadata_assignment: bool,
        include_embedding_assignment: bool,
        include_context_summary_assignment: bool,
        include_embedding_vector_assignment: bool,
        include_embedding_model_assignment: bool,
        include_embedding_dimensions_assignment: bool,
        include_embedded_at_assignment: bool,
        parameters: QueryParams,
    ) -> Result<()> {
        let update_query = raw_queries::update_temporal_node_query(
            record_id,
            include_parent_assignment,
            include_source_metadata_assignment,
            include_embedding_assignment,
            include_context_summary_assignment,
            include_embedding_vector_assignment,
            include_embedding_model_assignment,
            include_embedding_dimensions_assignment,
            include_embedded_at_assignment,
        );

        self.client.raw_query(&update_query, parameters).await?;
        Ok(())
    }
}

#[async_trait]
impl NodeStoreInitializer for SurrealDbNodeStore {
    async fn initialize_async(&self) -> Result<()> {
        self.client
            .raw_query(raw_queries::INIT_SCHEMA_QUERY, QueryParams::new())
            .await?;
        self.backfill_missing_tenant_ids_async().await?;
        Ok(())
    }
}

#[async_trait]
impl NodeStore for SurrealDbNodeStore {
    async fn query_nodes_async(&self, query: NodeQuery) -> Result<Vec<SttpNode>> {
        let capped_limit = query.limit.max(1);
        let mut clauses = Vec::new();
        let mut parameters = QueryParams::new();

        if let Some(session_id) = query.session_id.as_ref().filter(|s| !s.trim().is_empty()) {
            clauses
                .push("(tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')".to_string());
            parameters.insert(
                "tenant_id".to_string(),
                json!(derive_tenant_id_from_session(session_id)),
            );
            clauses.push("session_id = $session_id".to_string());
            parameters.insert("session_id".to_string(), json!(session_id));
        }

        let additional_predicate = build_retrieval_predicate(
            query.from_utc,
            query.to_utc,
            query.tiers.as_deref(),
            &mut parameters,
        );
        if !additional_predicate.is_empty() {
            clauses.push(additional_predicate);
        }

        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };

        let query_text = raw_queries::query_nodes_query(&where_clause, capped_limit);
        let rows = self.client.raw_query(&query_text, parameters).await?;
        let records: Vec<SurrealNodeRecord> = decode_rows(rows)?;

        Ok(records.into_iter().map(map_to_node).collect())
    }

    async fn upsert_node_async(&self, node: SttpNode) -> Result<NodeUpsertResult> {
        let mut candidate = node;
        let compression_avec_to_use = match candidate.compression_avec {
            Some(avec) if avec.psi() != 0.0 => avec,
            _ => candidate.model_avec,
        };

        let sync_key = if candidate.sync_key.trim().is_empty() {
            candidate.canonical_sync_key()
        } else {
            candidate.sync_key.trim().to_string()
        };

        let updated_at = candidate.updated_at;
        candidate.sync_key = sync_key.clone();
        candidate.updated_at = updated_at;

        let include_parent_assignment = candidate.parent_node_id.is_some();
        let include_source_metadata_assignment = candidate.source_metadata.is_some();
        let include_context_summary_assignment = candidate.context_summary.is_some();
        let include_embedding_vector_assignment = candidate.embedding.is_some();
        let include_embedding_model_assignment = candidate.embedding_model.is_some();
        let include_embedding_dimensions_assignment = candidate.embedding_dimensions.is_some();
        let include_embedding_assignment = include_context_summary_assignment
            || include_embedding_vector_assignment
            || include_embedding_model_assignment
            || include_embedding_dimensions_assignment
            || candidate.embedded_at.is_some();
        let include_embedded_at_assignment = candidate.embedded_at.is_some();

        let tenant_id = derive_tenant_id_from_session(&candidate.session_id);

        let mut parameters = QueryParams::new();

        parameters.insert("tenant_id".to_string(), json!(&tenant_id));
        parameters.insert("session_id".to_string(), json!(&candidate.session_id));
        parameters.insert("raw".to_string(), json!(&candidate.raw));
        parameters.insert("tier".to_string(), json!(&candidate.tier));
        parameters.insert(
            "timestamp".to_string(),
            json!(candidate.timestamp.to_rfc3339()),
        );
        parameters.insert(
            "compression_depth".to_string(),
            json!(candidate.compression_depth),
        );
        parameters.insert("sync_key".to_string(), json!(&sync_key));
        parameters.insert("updated_at".to_string(), json!(updated_at.to_rfc3339()));
        if let Some(metadata) = candidate.source_metadata.clone() {
            parameters.insert(
                "source_metadata".to_string(),
                serde_json::to_value(metadata).unwrap_or(Value::Null),
            );
        }
        parameters.insert(
            "context_summary".to_string(),
            candidate
                .context_summary
                .clone()
                .map_or(Value::Null, |value| json!(value)),
        );
        parameters.insert(
            "embedding".to_string(),
            candidate
                .embedding
                .clone()
                .map_or(Value::Null, |value| json!(value)),
        );
        parameters.insert(
            "embedding_model".to_string(),
            candidate
                .embedding_model
                .clone()
                .map_or(Value::Null, |value| json!(value)),
        );
        parameters.insert(
            "embedding_dimensions".to_string(),
            candidate
                .embedding_dimensions
                .map_or(Value::Null, |value| json!(value)),
        );
        parameters.insert(
            "embedded_at".to_string(),
            candidate
                .embedded_at
                .map(|value| value.to_rfc3339())
                .map_or(Value::Null, |value| json!(value)),
        );
        parameters.insert("psi".to_string(), json!(candidate.psi));
        parameters.insert("rho".to_string(), json!(candidate.rho));
        parameters.insert("kappa".to_string(), json!(candidate.kappa));
        parameters.insert(
            "user_stability".to_string(),
            json!(candidate.user_avec.stability),
        );
        parameters.insert(
            "user_friction".to_string(),
            json!(candidate.user_avec.friction),
        );
        parameters.insert("user_logic".to_string(), json!(candidate.user_avec.logic));
        parameters.insert(
            "user_autonomy".to_string(),
            json!(candidate.user_avec.autonomy),
        );
        parameters.insert("user_psi".to_string(), json!(candidate.user_avec.psi()));
        parameters.insert(
            "model_stability".to_string(),
            json!(candidate.model_avec.stability),
        );
        parameters.insert(
            "model_friction".to_string(),
            json!(candidate.model_avec.friction),
        );
        parameters.insert("model_logic".to_string(), json!(candidate.model_avec.logic));
        parameters.insert(
            "model_autonomy".to_string(),
            json!(candidate.model_avec.autonomy),
        );
        parameters.insert("model_psi".to_string(), json!(candidate.model_avec.psi()));
        parameters.insert(
            "comp_stability".to_string(),
            json!(compression_avec_to_use.stability),
        );
        parameters.insert(
            "comp_friction".to_string(),
            json!(compression_avec_to_use.friction),
        );
        parameters.insert(
            "comp_logic".to_string(),
            json!(compression_avec_to_use.logic),
        );
        parameters.insert(
            "comp_autonomy".to_string(),
            json!(compression_avec_to_use.autonomy),
        );
        parameters.insert("comp_psi".to_string(), json!(compression_avec_to_use.psi()));

        if let Some(parent_node_id) = candidate.parent_node_id.clone() {
            parameters.insert("parent_node_id".to_string(), json!(parent_node_id));
        }

        if let Some(existing) = self
            .find_existing_node_exact_async(&tenant_id, &candidate.session_id, &sync_key)
            .await?
            .or(self
                .find_existing_node_any_tenant_async(&candidate.session_id, &sync_key)
                .await?)
        {
            let existing_id = normalize_record_id(existing.id.clone(), "temporal_node")
                .and_then(|record_id| normalize_temporal_node_id(&record_id))
                .ok_or_else(|| anyhow!("existing node record id was invalid"))?;

            if existing_node_matches_candidate(&existing, &candidate) {
                return Ok(NodeUpsertResult {
                    node_id: existing_id,
                    sync_key,
                    status: NodeUpsertStatus::Duplicate,
                    updated_at,
                });
            }

            self.update_existing_temporal_node_async(
                &existing_id,
                include_parent_assignment,
                include_source_metadata_assignment,
                include_embedding_assignment,
                include_context_summary_assignment,
                include_embedding_vector_assignment,
                include_embedding_model_assignment,
                include_embedding_dimensions_assignment,
                include_embedded_at_assignment,
                parameters.clone(),
            )
            .await?;

            return Ok(NodeUpsertResult {
                node_id: existing_id,
                sync_key,
                status: NodeUpsertStatus::Updated,
                updated_at,
            });
        }

        let record_id = Uuid::new_v4().simple().to_string();
        let query_text = raw_queries::create_temporal_node_query(
            &record_id,
            include_parent_assignment,
            include_source_metadata_assignment,
            include_embedding_assignment,
            include_context_summary_assignment,
            include_embedding_vector_assignment,
            include_embedding_model_assignment,
            include_embedding_dimensions_assignment,
            include_embedded_at_assignment,
        );

        if let Err(create_err) = self.client.raw_query(&query_text, parameters.clone()).await {
            if !is_sync_identity_conflict(&create_err) {
                return Err(create_err);
            }

            if let Some(conflict_record_id) = extract_conflict_record_id(&create_err) {
                self.update_existing_temporal_node_async(
                    &conflict_record_id,
                    include_parent_assignment,
                    include_source_metadata_assignment,
                    include_embedding_assignment,
                    include_context_summary_assignment,
                    include_embedding_vector_assignment,
                    include_embedding_model_assignment,
                    include_embedding_dimensions_assignment,
                    include_embedded_at_assignment,
                    parameters.clone(),
                )
                .await?;

                return Ok(NodeUpsertResult {
                    node_id: conflict_record_id,
                    sync_key,
                    status: NodeUpsertStatus::Updated,
                    updated_at,
                });
            }

            let Some(existing) = self
                .find_existing_node_exact_async(&tenant_id, &candidate.session_id, &sync_key)
                .await?
                .or(self
                    .find_existing_node_any_tenant_async(&candidate.session_id, &sync_key)
                    .await?)
            else {
                return Err(create_err);
            };

            let existing_id = normalize_record_id(existing.id.clone(), "temporal_node")
                .and_then(|value| normalize_temporal_node_id(&value))
                .ok_or_else(|| anyhow!("existing node record id was invalid"))?;

            if !existing_node_matches_candidate(&existing, &candidate) {
                self.update_existing_temporal_node_async(
                    &existing_id,
                    include_parent_assignment,
                    include_source_metadata_assignment,
                    include_embedding_assignment,
                    include_context_summary_assignment,
                    include_embedding_vector_assignment,
                    include_embedding_model_assignment,
                    include_embedding_dimensions_assignment,
                    include_embedded_at_assignment,
                    parameters,
                )
                .await?;

                return Ok(NodeUpsertResult {
                    node_id: existing_id,
                    sync_key,
                    status: NodeUpsertStatus::Updated,
                    updated_at,
                });
            }

            return Ok(NodeUpsertResult {
                node_id: existing_id,
                sync_key,
                status: NodeUpsertStatus::Duplicate,
                updated_at,
            });
        }

        Ok(NodeUpsertResult {
            node_id: record_id,
            sync_key,
            status: NodeUpsertStatus::Created,
            updated_at,
        })
    }

    async fn get_by_resonance_async(
        &self,
        session_id: &str,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let mut parameters = QueryParams::new();
        let additional_predicate =
            build_retrieval_predicate(from_utc, to_utc, tiers, &mut parameters);
        let query_text = raw_queries::get_by_resonance_query(
            current_avec.stability,
            current_avec.friction,
            current_avec.logic,
            current_avec.autonomy,
            &additional_predicate,
            limit.max(1),
        );
        parameters.insert(
            "tenant_id".to_string(),
            json!(derive_tenant_id_from_session(session_id)),
        );
        parameters.insert("session_id".to_string(), json!(session_id));

        let rows = self.client.raw_query(&query_text, parameters).await?;
        let records: Vec<SurrealNodeRecord> = decode_rows(rows)?;

        Ok(records.into_iter().map(map_to_node).collect())
    }

    async fn get_by_resonance_global_async(
        &self,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let mut parameters = QueryParams::new();
        let additional_predicate =
            build_retrieval_predicate(from_utc, to_utc, tiers, &mut parameters);
        let query_text = raw_queries::get_by_resonance_global_query(
            current_avec.stability,
            current_avec.friction,
            current_avec.logic,
            current_avec.autonomy,
            &additional_predicate,
            limit.max(1),
        );
        let rows = self.client.raw_query(&query_text, parameters).await?;
        let records: Vec<SurrealNodeRecord> = decode_rows(rows)?;

        Ok(records.into_iter().map(map_to_node).collect())
    }

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
        let candidate_limit = limit.max(1).saturating_mul(5);
        let mut candidates = self
            .get_by_resonance_async(
                session_id,
                current_avec,
                from_utc,
                to_utc,
                tiers,
                candidate_limit,
            )
            .await?;

        rank_nodes_hybrid(&mut candidates, current_avec, query_embedding, alpha, beta);
        candidates.truncate(limit.max(1));
        Ok(candidates)
    }

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
        let candidate_limit = limit.max(1).saturating_mul(5);
        let mut candidates = self
            .get_by_resonance_global_async(current_avec, from_utc, to_utc, tiers, candidate_limit)
            .await?;

        rank_nodes_hybrid(&mut candidates, current_avec, query_embedding, alpha, beta);
        candidates.truncate(limit.max(1));
        Ok(candidates)
    }

    async fn list_nodes_async(
        &self,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<SttpNode>> {
        self.query_nodes_async(NodeQuery {
            limit: limit.clamp(1, 200),
            session_id: session_id.map(|s| s.to_string()),
            from_utc: None,
            to_utc: None,
            tiers: None,
        })
        .await
    }

    async fn get_last_avec_async(&self, session_id: &str) -> Result<Option<AvecState>> {
        let mut parameters = QueryParams::new();
        parameters.insert(
            "tenant_id".to_string(),
            json!(derive_tenant_id_from_session(session_id)),
        );
        parameters.insert("session_id".to_string(), json!(session_id));

        let rows = self
            .client
            .raw_query(raw_queries::GET_LAST_AVEC_QUERY, parameters)
            .await?;
        let records: Vec<SurrealAvecRecord> = decode_rows(rows)?;

        Ok(records.first().map(|last| AvecState {
            stability: last.stability,
            friction: last.friction,
            logic: last.logic,
            autonomy: last.autonomy,
        }))
    }

    async fn get_trigger_history_async(&self, session_id: &str) -> Result<Vec<String>> {
        let mut parameters = QueryParams::new();
        parameters.insert(
            "tenant_id".to_string(),
            json!(derive_tenant_id_from_session(session_id)),
        );
        parameters.insert("session_id".to_string(), json!(session_id));

        let rows = self
            .client
            .raw_query(raw_queries::GET_TRIGGER_HISTORY_QUERY, parameters)
            .await?;
        let records: Vec<SurrealTriggerRecord> = decode_rows(rows)?;

        Ok(records.into_iter().map(|record| record.trigger).collect())
    }

    async fn store_calibration_async(
        &self,
        session_id: &str,
        avec: AvecState,
        trigger: &str,
    ) -> Result<()> {
        let mut parameters = QueryParams::new();
        parameters.insert(
            "tenant_id".to_string(),
            json!(derive_tenant_id_from_session(session_id)),
        );
        parameters.insert("session_id".to_string(), json!(session_id));
        parameters.insert("stability".to_string(), json!(avec.stability));
        parameters.insert("friction".to_string(), json!(avec.friction));
        parameters.insert("logic".to_string(), json!(avec.logic));
        parameters.insert("autonomy".to_string(), json!(avec.autonomy));
        parameters.insert("psi".to_string(), json!(avec.psi()));
        parameters.insert("trigger".to_string(), json!(trigger));
        parameters.insert("created_at".to_string(), json!(Utc::now().to_rfc3339()));

        self.client
            .raw_query(raw_queries::STORE_CALIBRATION_QUERY, parameters)
            .await?;

        Ok(())
    }

    async fn query_changes_since_async(
        &self,
        session_id: &str,
        cursor: Option<SyncCursor>,
        limit: usize,
    ) -> Result<ChangeQueryResult> {
        let capped_limit = limit.max(1);
        let query_text = raw_queries::query_changes_since_query(capped_limit + 1);
        let mut parameters = QueryParams::new();
        parameters.insert(
            "tenant_id".to_string(),
            json!(derive_tenant_id_from_session(session_id)),
        );
        parameters.insert("session_id".to_string(), json!(session_id));
        parameters.insert("include_cursor".to_string(), json!(cursor.is_some()));
        parameters.insert(
            "cursor_updated_at".to_string(),
            cursor
                .as_ref()
                .map(|cursor| json!(cursor.updated_at.to_rfc3339()))
                .unwrap_or(Value::Null),
        );
        parameters.insert(
            "cursor_sync_key".to_string(),
            cursor
                .as_ref()
                .map(|cursor| json!(&cursor.sync_key))
                .unwrap_or(Value::Null),
        );

        let rows = self.client.raw_query(&query_text, parameters).await?;
        let mut records: Vec<SurrealNodeRecord> = decode_rows(rows)?;
        let has_more = records.len() > capped_limit;
        if has_more {
            records.truncate(capped_limit);
        }

        let nodes = records.into_iter().map(map_to_node).collect::<Vec<_>>();
        let next_cursor = nodes.last().map(|node| SyncCursor {
            updated_at: node.updated_at,
            sync_key: node.sync_key.clone(),
        });

        Ok(ChangeQueryResult {
            nodes,
            next_cursor,
            has_more,
        })
    }

    async fn get_checkpoint_async(
        &self,
        session_id: &str,
        connector_id: &str,
    ) -> Result<Option<SyncCheckpoint>> {
        let mut parameters = QueryParams::new();
        parameters.insert(
            "tenant_id".to_string(),
            json!(derive_tenant_id_from_session(session_id)),
        );
        parameters.insert("session_id".to_string(), json!(session_id));
        parameters.insert("connector_id".to_string(), json!(connector_id));

        let rows = self
            .client
            .raw_query(raw_queries::GET_SYNC_CHECKPOINT_QUERY, parameters)
            .await?;
        let records: Vec<SurrealCheckpointRecord> = decode_rows(rows)?;

        Ok(records.first().map(map_to_checkpoint))
    }

    async fn put_checkpoint_async(&self, checkpoint: SyncCheckpoint) -> Result<()> {
        let tenant_id = derive_tenant_id_from_session(&checkpoint.session_id);
        let record_id =
            checkpoint_record_id(&tenant_id, &checkpoint.session_id, &checkpoint.connector_id);
        let include_metadata_assignment = checkpoint.metadata.is_some();
        let mut parameters = QueryParams::new();
        parameters.insert("tenant_id".to_string(), json!(tenant_id));
        parameters.insert("session_id".to_string(), json!(&checkpoint.session_id));
        parameters.insert("connector_id".to_string(), json!(&checkpoint.connector_id));
        parameters.insert(
            "cursor_updated_at".to_string(),
            checkpoint
                .cursor
                .as_ref()
                .map(|cursor| json!(cursor.updated_at.to_rfc3339()))
                .unwrap_or(Value::Null),
        );
        parameters.insert(
            "cursor_sync_key".to_string(),
            checkpoint
                .cursor
                .as_ref()
                .map(|cursor| json!(&cursor.sync_key))
                .unwrap_or(Value::Null),
        );
        if let Some(metadata) = checkpoint.metadata.clone() {
            parameters.insert(
                "metadata".to_string(),
                serde_json::to_value(metadata).unwrap_or(Value::Null),
            );
        }
        parameters.insert(
            "updated_at".to_string(),
            json!(checkpoint.updated_at.to_rfc3339()),
        );

        let query_text =
            raw_queries::upsert_sync_checkpoint_query(&record_id, include_metadata_assignment);
        self.client.raw_query(&query_text, parameters).await?;
        Ok(())
    }

    async fn batch_rekey_scopes_async(
        &self,
        node_ids: Vec<String>,
        target_tenant_id: &str,
        target_session_id: &str,
        dry_run: bool,
        allow_merge: bool,
    ) -> Result<BatchRekeyResult> {
        if node_ids.is_empty() {
            return Err(anyhow!("at least one node id is required"));
        }

        if target_session_id.trim().is_empty() {
            return Err(anyhow!("target session id cannot be empty"));
        }

        let target_tenant_id = normalize_tenant_id(Some(target_tenant_id));
        let normalized_node_ids = node_ids
            .into_iter()
            .filter_map(|node_id| normalize_temporal_node_id(&node_id))
            .collect::<Vec<_>>();

        if normalized_node_ids.is_empty() {
            return Err(anyhow!("no valid node ids were provided"));
        }

        let mut missing_node_ids = Vec::new();
        let mut scope_keys = BTreeSet::new();

        for node_id in &normalized_node_ids {
            let mut parameters = QueryParams::new();
            parameters.insert("node_id".to_string(), json!(node_id));

            let rows = self
                .client
                .raw_query(raw_queries::SELECT_SCOPE_BY_NODE_ID_QUERY, parameters)
                .await?;
            let anchors: Vec<ScopeAnchorRecord> = decode_rows(rows)?;

            let Some(anchor) = anchors.first() else {
                missing_node_ids.push(node_id.clone());
                continue;
            };

            if anchor.session_id.trim().is_empty() {
                missing_node_ids.push(node_id.clone());
                continue;
            }

            scope_keys.insert(ScopeKey {
                tenant_id: normalize_tenant_id(anchor.tenant_id.as_deref()),
                session_id: anchor.session_id.clone(),
            });
        }

        let mut scope_results = Vec::new();
        let mut temporal_nodes_updated = 0usize;
        let mut calibrations_updated = 0usize;

        for scope in scope_keys {
            let source_include_legacy = includes_legacy_tenant_bucket(&scope.tenant_id);
            let temporal_nodes = self
                .count_scope_rows_async(
                    raw_queries::COUNT_TEMPORAL_SCOPE_QUERY,
                    &scope.session_id,
                    &scope.tenant_id,
                    source_include_legacy,
                )
                .await?;
            let calibrations = self
                .count_scope_rows_async(
                    raw_queries::COUNT_CALIBRATION_SCOPE_QUERY,
                    &scope.session_id,
                    &scope.tenant_id,
                    source_include_legacy,
                )
                .await?;

            let same_scope =
                scope.tenant_id == target_tenant_id && scope.session_id == target_session_id;

            let target_include_legacy = includes_legacy_tenant_bucket(&target_tenant_id);
            let target_temporal_nodes = if same_scope {
                0
            } else {
                self.count_scope_rows_async(
                    raw_queries::COUNT_TEMPORAL_SCOPE_QUERY,
                    target_session_id,
                    &target_tenant_id,
                    target_include_legacy,
                )
                .await?
            };
            let target_calibrations = if same_scope {
                0
            } else {
                self.count_scope_rows_async(
                    raw_queries::COUNT_CALIBRATION_SCOPE_QUERY,
                    target_session_id,
                    &target_tenant_id,
                    target_include_legacy,
                )
                .await?
            };

            let conflict = !allow_merge
                && !same_scope
                && (target_temporal_nodes > 0 || target_calibrations > 0);

            let mut applied = false;
            let message = if same_scope {
                Some("source and target scopes are identical".to_string())
            } else if conflict {
                Some(
                    "target scope already contains rows; set allow_merge=true to override"
                        .to_string(),
                )
            } else {
                if !dry_run {
                    self.apply_scope_rekey_async(
                        &scope.session_id,
                        &scope.tenant_id,
                        target_session_id,
                        &target_tenant_id,
                    )
                    .await?;
                    applied = true;
                    temporal_nodes_updated += temporal_nodes;
                    calibrations_updated += calibrations;
                }
                None
            };

            scope_results.push(ScopeRekeyResult {
                source_tenant_id: scope.tenant_id,
                source_session_id: scope.session_id,
                target_tenant_id: target_tenant_id.clone(),
                target_session_id: target_session_id.to_string(),
                temporal_nodes,
                calibrations,
                target_temporal_nodes,
                target_calibrations,
                applied,
                conflict,
                message,
            });
        }

        Ok(BatchRekeyResult {
            dry_run,
            requested_node_ids: normalized_node_ids.len(),
            resolved_node_ids: normalized_node_ids
                .len()
                .saturating_sub(missing_node_ids.len()),
            missing_node_ids,
            scopes: scope_results,
            temporal_nodes_updated,
            calibrations_updated,
        })
    }
}

fn map_to_node(record: SurrealNodeRecord) -> SttpNode {
    let _ = (
        record.user_psi,
        record.model_psi,
        record.comp_psi,
        record.resonance_delta,
    );

    let timestamp = parse_timestamp(&record.timestamp);
    let updated_at = record
        .updated_at
        .as_deref()
        .map(parse_timestamp)
        .unwrap_or(timestamp);
    let sync_key = record.sync_key.unwrap_or_default();

    let mut node = SttpNode {
        raw: record.raw,
        session_id: record.session_id,
        tier: record.tier,
        timestamp,
        compression_depth: record.compression_depth,
        parent_node_id: record.parent_node_id,
        sync_key,
        updated_at,
        source_metadata: record.source_metadata,
        context_summary: record.context_summary,
        embedding: record.embedding,
        embedding_model: record.embedding_model,
        embedding_dimensions: record.embedding_dimensions,
        embedded_at: record.embedded_at.as_deref().map(parse_timestamp),
        psi: record.psi as f32,
        rho: record.rho as f32,
        kappa: record.kappa as f32,
        user_avec: AvecState {
            stability: record.user_stability as f32,
            friction: record.user_friction as f32,
            logic: record.user_logic as f32,
            autonomy: record.user_autonomy as f32,
        },
        model_avec: AvecState {
            stability: record.model_stability as f32,
            friction: record.model_friction as f32,
            logic: record.model_logic as f32,
            autonomy: record.model_autonomy as f32,
        },
        compression_avec: Some(AvecState {
            stability: record.comp_stability as f32,
            friction: record.comp_friction as f32,
            logic: record.comp_logic as f32,
            autonomy: record.comp_autonomy as f32,
        }),
    };

    if node.sync_key.trim().is_empty() {
        node.sync_key = node.canonical_sync_key();
    }

    node
}

fn rank_nodes_hybrid(
    nodes: &mut [SttpNode],
    current_avec: AvecState,
    query_embedding: Option<&[f32]>,
    alpha: f32,
    beta: f32,
) {
    let alpha = alpha.clamp(0.0, 1.0);
    let beta = beta.clamp(0.0, 1.0);

    nodes.sort_by(|left, right| {
        let left_score = hybrid_score(left, current_avec, query_embedding, alpha, beta);
        let right_score = hybrid_score(right, current_avec, query_embedding, alpha, beta);

        right_score
            .total_cmp(&left_score)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
}

fn hybrid_score(
    node: &SttpNode,
    target_avec: AvecState,
    query_embedding: Option<&[f32]>,
    alpha: f32,
    beta: f32,
) -> f32 {
    let resonance = 1.0 - avec_distance(&node.model_avec, &target_avec);
    let resonance = resonance.clamp(0.0, 1.0);

    let semantic = match (query_embedding, node.embedding.as_deref()) {
        (Some(query), Some(node_vec)) => {
            cosine_similarity(query, node_vec).map(|v| (v + 1.0) / 2.0)
        }
        _ => None,
    };

    match semantic {
        Some(value) => (alpha * resonance + beta * value).clamp(0.0, 1.0),
        None => resonance,
    }
}

fn avec_distance(left: &AvecState, right: &AvecState) -> f32 {
    ((left.stability - right.stability).abs()
        + (left.friction - right.friction).abs()
        + (left.logic - right.logic).abs()
        + (left.autonomy - right.autonomy).abs())
        / 4.0
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return None;
    }

    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;

    for (l, r) in left.iter().zip(right.iter()) {
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }

    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return None;
    }

    Some(dot / (left_norm.sqrt() * right_norm.sqrt()))
}

fn map_to_checkpoint(record: &SurrealCheckpointRecord) -> SyncCheckpoint {
    SyncCheckpoint {
        session_id: record.session_id.clone(),
        connector_id: record.connector_id.clone(),
        cursor: match (&record.cursor_updated_at, &record.cursor_sync_key) {
            (Some(updated_at), Some(sync_key)) if !sync_key.trim().is_empty() => Some(SyncCursor {
                updated_at: parse_timestamp(updated_at),
                sync_key: sync_key.clone(),
            }),
            _ => None,
        },
        updated_at: parse_timestamp(&record.updated_at),
        metadata: record.metadata.clone(),
    }
}

fn parse_timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn resolve_legacy_updated_at(primary: Option<&str>, fallback: Option<&str>) -> String {
    parse_optional_timestamp(primary)
        .or_else(|| parse_optional_timestamp(fallback))
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

fn parse_optional_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn decode_rows<T>(rows: Vec<serde_json::Value>) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    rows.into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<T>, _>>()
        .map_err(Into::into)
}

fn normalize_record_id(raw_id: Value, fallback_table: &str) -> Option<String> {
    match raw_id {
        Value::String(id) => {
            if id.trim().is_empty() {
                None
            } else {
                Some(id)
            }
        }
        Value::Object(map) => {
            let table_name = map
                .get("tb")
                .and_then(value_to_record_component)
                .unwrap_or_else(|| fallback_table.to_string());

            let id_component = map.get("id").and_then(value_to_record_component)?;
            Some(format!("{table_name}:{id_component}"))
        }
        _ => None,
    }
}

fn value_to_record_component(value: &Value) -> Option<String> {
    match value {
        Value::String(v) => Some(v.clone()),
        Value::Number(v) => Some(v.to_string()),
        Value::Bool(v) => Some(v.to_string()),
        _ => None,
    }
}

fn build_retrieval_predicate(
    from_utc: Option<DateTime<Utc>>,
    to_utc: Option<DateTime<Utc>>,
    tiers: Option<&[String]>,
    parameters: &mut QueryParams,
) -> String {
    let mut clauses = Vec::new();

    if let Some(from_utc) = from_utc {
        clauses.push("timestamp >= <datetime>$from_utc".to_string());
        parameters.insert("from_utc".to_string(), json!(from_utc.to_rfc3339()));
    }

    if let Some(to_utc) = to_utc {
        clauses.push("timestamp <= <datetime>$to_utc".to_string());
        parameters.insert("to_utc".to_string(), json!(to_utc.to_rfc3339()));
    }

    let normalized_tiers = tiers
        .unwrap_or(&[])
        .iter()
        .map(|tier| tier.trim().to_ascii_lowercase())
        .filter(|tier| !tier.is_empty())
        .collect::<Vec<_>>();

    if !normalized_tiers.is_empty() {
        let tier_clauses = normalized_tiers
            .iter()
            .enumerate()
            .map(|(idx, tier)| {
                let key = format!("tier_{idx}");
                parameters.insert(key.clone(), json!(tier));
                format!("string::lowercase(tier) = ${key}")
            })
            .collect::<Vec<_>>();

        clauses.push(format!("({})", tier_clauses.join(" OR ")));
    }

    clauses.join(" AND ")
}

fn derive_tenant_id_from_session(session_id: &str) -> String {
    session_id
        .strip_prefix(TENANT_SCOPE_PREFIX)
        .and_then(|remainder| remainder.split_once(TENANT_SCOPE_SEPARATOR))
        .map(|(tenant, _)| tenant)
        .filter(|tenant| !tenant.trim().is_empty())
        .unwrap_or(DEFAULT_TENANT)
        .to_string()
}

fn normalize_tenant_id(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|tenant| !tenant.is_empty())
        .unwrap_or(DEFAULT_TENANT)
        .to_string()
}

fn includes_legacy_tenant_bucket(tenant_id: &str) -> bool {
    tenant_id == DEFAULT_TENANT
}

fn normalize_temporal_node_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((table, id)) = trimmed.split_once(':') {
        if table != "temporal_node" {
            return None;
        }

        let id = id.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    } else {
        Some(trimmed.to_string())
    }
}

fn existing_node_matches_candidate(
    existing: &SurrealExistingNodeRecord,
    candidate: &SttpNode,
) -> bool {
    let existing_embedded_at = parse_optional_timestamp(existing.embedded_at.as_deref());

    normalize_metadata(existing.source_metadata.as_ref())
        == normalize_metadata(candidate.source_metadata.as_ref())
        && existing.context_summary == candidate.context_summary
        && existing.embedding == candidate.embedding
        && existing.embedding_model == candidate.embedding_model
        && existing.embedding_dimensions == candidate.embedding_dimensions
        && existing_embedded_at == candidate.embedded_at
}

fn is_sync_identity_conflict(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("idx_node_sync_identity") || message.contains("already contains")
}

fn extract_conflict_record_id(error: &anyhow::Error) -> Option<String> {
    let message = error.to_string();
    let marker = "with record `";
    let start = message.find(marker)? + marker.len();
    let remainder = &message[start..];
    let end = remainder.find('`')?;
    let raw_record = &remainder[..end];

    normalize_temporal_node_id(raw_record)
}

fn normalize_metadata(metadata: Option<&ConnectorMetadata>) -> Option<String> {
    metadata.and_then(|value| serde_json::to_string(value).ok())
}

fn normalize_legacy_sync_key(value: Option<&str>, node_id: &str) -> String {
    value
        .map(str::trim)
        .filter(|sync_key| !sync_key.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("legacy:{node_id}"))
}

fn checkpoint_record_id(tenant_id: &str, session_id: &str, connector_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tenant_id.as_bytes());
    hasher.update([0]);
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(connector_id.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
