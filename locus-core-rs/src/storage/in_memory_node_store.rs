use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::contracts::{NodeStore, NodeStoreInitializer};
use crate::domain::models::{
    AvecState, BatchRekeyResult, ChangeQueryResult, ConnectorMetadata, NodeQuery, NodeUpsertResult,
    NodeUpsertStatus, ScopeRekeyResult, SttpNode, SyncCheckpoint, SyncCursor,
};

const DEFAULT_TENANT: &str = "default";
const TENANT_SCOPE_PREFIX: &str = "tenant:";
const TENANT_SCOPE_SEPARATOR: &str = "::session:";

#[derive(Debug, Clone, Copy)]
struct CalibrationRecord {
    avec: AvecState,
}

fn normalize_tiers(tiers: Option<&[String]>) -> Option<Vec<String>> {
    let normalized = tiers
        .unwrap_or(&[])
        .iter()
        .map(|tier| tier.trim().to_ascii_lowercase())
        .filter(|tier| !tier.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}
#[derive(Debug, Default)]
pub struct InMemoryNodeStore {
    nodes: RwLock<Vec<(String, SttpNode)>>,
    calibrations: RwLock<Vec<(String, CalibrationRecord, String)>>,
    checkpoints: RwLock<Vec<SyncCheckpoint>>,
}

impl InMemoryNodeStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl NodeStoreInitializer for InMemoryNodeStore {
    async fn initialize_async(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl NodeStore for InMemoryNodeStore {
    async fn query_nodes_async(&self, query: NodeQuery) -> Result<Vec<SttpNode>> {
        let capped_limit = query.limit.max(1);
        let nodes = self.nodes.read().await;

        let mut result = nodes
            .iter()
            .map(|(_, node)| node)
            .filter(|n| {
                query
                    .session_id
                    .as_ref()
                    .map(|s| &n.session_id == s)
                    .unwrap_or(true)
            })
            .filter(|n| {
                query
                    .from_utc
                    .map(|from| n.timestamp >= from)
                    .unwrap_or(true)
            })
            .filter(|n| query.to_utc.map(|to| n.timestamp <= to).unwrap_or(true))
            .filter(|n| {
                query
                    .tiers
                    .as_ref()
                    .map(|tiers| tiers.iter().any(|tier| n.tier.eq_ignore_ascii_case(tier)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<_>>();

        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        result.truncate(capped_limit);

        Ok(result)
    }

    async fn upsert_node_async(&self, node: SttpNode) -> Result<NodeUpsertResult> {
        let mut candidate = node;
        let sync_key = if candidate.sync_key.trim().is_empty() {
            candidate.canonical_sync_key()
        } else {
            candidate.sync_key.trim().to_string()
        };

        let updated_at = candidate.updated_at;
        candidate.sync_key = sync_key.clone();
        candidate.updated_at = updated_at;

        let mut nodes = self.nodes.write().await;
        if let Some((node_id, existing)) = nodes.iter_mut().find(|(_, existing)| {
            existing.session_id == candidate.session_id && existing.sync_key == sync_key
        }) {
            let existing_metadata = normalize_metadata(existing.source_metadata.as_ref());
            let candidate_metadata = normalize_metadata(candidate.source_metadata.as_ref());

            if existing.raw != candidate.raw
                || existing.tier != candidate.tier
                || existing.timestamp != candidate.timestamp
                || existing.compression_depth != candidate.compression_depth
                || existing.parent_node_id != candidate.parent_node_id
                || existing.context_summary != candidate.context_summary
                || existing.embedding != candidate.embedding
                || existing.embedding_model != candidate.embedding_model
                || existing.embedding_dimensions != candidate.embedding_dimensions
                || existing.embedded_at != candidate.embedded_at
                || existing.user_avec != candidate.user_avec
                || existing.model_avec != candidate.model_avec
                || existing.compression_avec != candidate.compression_avec
                || (existing.rho - candidate.rho).abs() > f32::EPSILON
                || (existing.kappa - candidate.kappa).abs() > f32::EPSILON
                || (existing.psi - candidate.psi).abs() > f32::EPSILON
                || existing_metadata != candidate_metadata
            {
                *existing = candidate.clone();
                existing.updated_at = updated_at;
                return Ok(NodeUpsertResult {
                    node_id: node_id.clone(),
                    sync_key,
                    status: NodeUpsertStatus::Updated,
                    updated_at,
                });
            }

            return Ok(NodeUpsertResult {
                node_id: node_id.clone(),
                sync_key,
                status: NodeUpsertStatus::Duplicate,
                updated_at: existing.updated_at,
            });
        }

        let node_id = Uuid::new_v4().to_string();
        nodes.push((node_id.clone(), candidate));

        Ok(NodeUpsertResult {
            node_id,
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
        let mut result = self
            .query_nodes_async(NodeQuery {
                limit: usize::MAX,
                session_id: Some(session_id.to_string()),
                from_utc,
                to_utc,
                tiers: normalize_tiers(tiers),
            })
            .await?;

        result.sort_by(|a, b| {
            let ad = avec_distance(&a.model_avec, &current_avec);
            let bd = avec_distance(&b.model_avec, &current_avec);
            ad.total_cmp(&bd)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        result.truncate(limit.max(1));

        Ok(result)
    }

    async fn get_by_resonance_global_async(
        &self,
        current_avec: AvecState,
        from_utc: Option<DateTime<Utc>>,
        to_utc: Option<DateTime<Utc>>,
        tiers: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SttpNode>> {
        let mut result = self
            .query_nodes_async(NodeQuery {
                limit: usize::MAX,
                session_id: None,
                from_utc,
                to_utc,
                tiers: normalize_tiers(tiers),
            })
            .await?;

        result.sort_by(|a, b| {
            let ad = avec_distance(&a.model_avec, &current_avec);
            let bd = avec_distance(&b.model_avec, &current_avec);
            ad.total_cmp(&bd)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        result.truncate(limit.max(1));

        Ok(result)
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
        let mut result = self
            .query_nodes_async(NodeQuery {
                limit: usize::MAX,
                session_id: Some(session_id.to_string()),
                from_utc,
                to_utc,
                tiers: normalize_tiers(tiers),
            })
            .await?;

        rank_nodes_hybrid(&mut result, current_avec, query_embedding, alpha, beta);
        result.truncate(limit.max(1));
        Ok(result)
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
        let mut result = self
            .query_nodes_async(NodeQuery {
                limit: usize::MAX,
                session_id: None,
                from_utc,
                to_utc,
                tiers: normalize_tiers(tiers),
            })
            .await?;

        rank_nodes_hybrid(&mut result, current_avec, query_embedding, alpha, beta);
        result.truncate(limit.max(1));
        Ok(result)
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
        let calibrations = self.calibrations.read().await;
        let last = calibrations
            .iter()
            .rev()
            .find(|(sid, _, _)| sid == session_id)
            .map(|(_, record, _)| record.avec);
        Ok(last)
    }

    async fn get_trigger_history_async(&self, session_id: &str) -> Result<Vec<String>> {
        let calibrations = self.calibrations.read().await;
        let history = calibrations
            .iter()
            .filter(|(sid, _, _)| sid == session_id)
            .map(|(_, _, trigger)| trigger.clone())
            .collect::<Vec<_>>();

        Ok(history)
    }

    async fn store_calibration_async(
        &self,
        session_id: &str,
        avec: AvecState,
        trigger: &str,
    ) -> Result<()> {
        let mut calibrations = self.calibrations.write().await;
        calibrations.push((
            session_id.to_string(),
            CalibrationRecord { avec },
            trigger.to_string(),
        ));
        Ok(())
    }

    async fn query_changes_since_async(
        &self,
        session_id: &str,
        cursor: Option<SyncCursor>,
        limit: usize,
    ) -> Result<ChangeQueryResult> {
        let capped_limit = limit.max(1);
        let nodes = self.nodes.read().await;

        let mut changes = nodes
            .iter()
            .map(|(_, node)| node)
            .filter(|node| node.session_id == session_id)
            .filter(|node| match cursor.as_ref() {
                Some(cursor) => {
                    node.updated_at > cursor.updated_at
                        || (node.updated_at == cursor.updated_at && node.sync_key > cursor.sync_key)
                }
                None => true,
            })
            .cloned()
            .collect::<Vec<_>>();

        changes.sort_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .then_with(|| left.sync_key.cmp(&right.sync_key))
        });

        let has_more = changes.len() > capped_limit;
        if has_more {
            changes.truncate(capped_limit);
        }

        let next_cursor = changes.last().map(|node| SyncCursor {
            updated_at: node.updated_at,
            sync_key: node.sync_key.clone(),
        });

        Ok(ChangeQueryResult {
            nodes: changes,
            next_cursor,
            has_more,
        })
    }

    async fn get_checkpoint_async(
        &self,
        session_id: &str,
        connector_id: &str,
    ) -> Result<Option<SyncCheckpoint>> {
        let checkpoints = self.checkpoints.read().await;
        Ok(checkpoints
            .iter()
            .find(|checkpoint| {
                checkpoint.session_id == session_id && checkpoint.connector_id == connector_id
            })
            .cloned())
    }

    async fn put_checkpoint_async(&self, checkpoint: SyncCheckpoint) -> Result<()> {
        let mut checkpoints = self.checkpoints.write().await;
        if let Some(existing) = checkpoints.iter_mut().find(|existing| {
            existing.session_id == checkpoint.session_id
                && existing.connector_id == checkpoint.connector_id
        }) {
            *existing = checkpoint;
        } else {
            checkpoints.push(checkpoint);
        }

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

        let target_tenant_id = normalize_tenant_id(target_tenant_id);
        if target_session_id.trim().is_empty() {
            return Err(anyhow!("target session id cannot be empty"));
        }

        let normalized_node_ids = node_ids
            .into_iter()
            .filter_map(|node_id| normalize_temporal_node_id(&node_id))
            .collect::<Vec<_>>();

        if normalized_node_ids.is_empty() {
            return Err(anyhow!("no valid node ids were provided"));
        }

        let nodes_snapshot = self.nodes.read().await.clone();
        let calibrations_snapshot = self.calibrations.read().await.clone();

        let mut missing_node_ids = Vec::new();
        let mut scopes = Vec::new();

        for node_id in &normalized_node_ids {
            let Some((_, node)) = nodes_snapshot.iter().find(|(id, _)| id == node_id) else {
                missing_node_ids.push(node_id.clone());
                continue;
            };

            let tenant_id = derive_tenant_id_from_session(&node.session_id);
            if !scopes.iter().any(|(tenant, session): &(String, String)| {
                tenant == &tenant_id && session == &node.session_id
            }) {
                scopes.push((tenant_id, node.session_id.clone()));
            }
        }

        let mut scope_results = Vec::new();
        let mut temporal_nodes_updated = 0usize;
        let mut calibrations_updated = 0usize;
        let mut sources_to_apply = Vec::new();

        for (source_tenant_id, source_session_id) in scopes {
            let same_scope =
                source_tenant_id == target_tenant_id && source_session_id == target_session_id;

            let temporal_nodes = nodes_snapshot
                .iter()
                .filter(|(_, node)| node.session_id == source_session_id)
                .count();
            let calibrations = calibrations_snapshot
                .iter()
                .filter(|(session_id, _, _)| session_id == &source_session_id)
                .count();

            let target_temporal_nodes = if same_scope {
                0
            } else {
                nodes_snapshot
                    .iter()
                    .filter(|(_, node)| node.session_id == target_session_id)
                    .count()
            };
            let target_calibrations = if same_scope {
                0
            } else {
                calibrations_snapshot
                    .iter()
                    .filter(|(session_id, _, _)| session_id == target_session_id)
                    .count()
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
                    applied = true;
                    temporal_nodes_updated += temporal_nodes;
                    calibrations_updated += calibrations;
                    sources_to_apply.push(source_session_id.clone());
                }
                None
            };

            scope_results.push(ScopeRekeyResult {
                source_tenant_id,
                source_session_id,
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

        if !dry_run && !sources_to_apply.is_empty() {
            let mut nodes = self.nodes.write().await;
            for (_, node) in nodes.iter_mut() {
                if sources_to_apply
                    .iter()
                    .any(|source| source == &node.session_id)
                {
                    node.session_id = target_session_id.to_string();
                }
            }

            let mut calibrations = self.calibrations.write().await;
            for (session_id, _, _) in calibrations.iter_mut() {
                if sources_to_apply.iter().any(|source| source == session_id) {
                    *session_id = target_session_id.to_string();
                }
            }
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

fn parse_scoped_session_id(session_id: &str) -> Option<(&str, &str)> {
    let remainder = session_id.strip_prefix(TENANT_SCOPE_PREFIX)?;
    remainder.split_once(TENANT_SCOPE_SEPARATOR)
}

fn derive_tenant_id_from_session(session_id: &str) -> String {
    parse_scoped_session_id(session_id)
        .map(|(tenant, _)| tenant)
        .filter(|tenant| !tenant.trim().is_empty())
        .unwrap_or(DEFAULT_TENANT)
        .to_string()
}

fn normalize_tenant_id(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        DEFAULT_TENANT.to_string()
    } else {
        trimmed.to_string()
    }
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

fn normalize_metadata(metadata: Option<&ConnectorMetadata>) -> Option<String> {
    metadata.and_then(|value| serde_json::to_string(value).ok())
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
