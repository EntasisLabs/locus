use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore};
use locus_core_rs::domain::models::{
    NodeDeleteRecord, NodeDeleteRequest, NodeDeleteResult, NodeDeleteStatus, NodeQuery,
    SessionPurgeRequest, SttpNode,
};
use locus_core_rs::storage::derive_tenant_id_from_session;

use crate::application::memory_filters::{
    build_session_filter, node_matches_common_filters, resolve_indexed_sync_keys,
};
use crate::application::memory_graph::graph_node_id;
use crate::domain::evict::{
    InboundReferencesPreview, MemoryEvictMode, MemoryEvictRecord, MemoryEvictRequest,
    MemoryEvictResult,
};
use crate::domain::memory::{clamp_batch_size, clamp_nodes};

const TENANT_SCAN_LIMIT: usize = 5000;

#[derive(Debug, Clone)]
struct EvictCandidate {
    store_node_id: String,
    sync_key: String,
    graph_id: String,
}

pub struct MemoryEvictService {
    store: Arc<dyn NodeStore>,
    semantic_index: Option<Arc<dyn SemanticIndexStore>>,
}

impl MemoryEvictService {
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self {
            store,
            semantic_index: None,
        }
    }

    pub fn with_semantic_index(
        mut self,
        semantic_index: Arc<dyn SemanticIndexStore>,
    ) -> Self {
        self.semantic_index = Some(semantic_index);
        self
    }

    pub async fn execute(&self, request: &MemoryEvictRequest) -> Result<MemoryEvictResult> {
        if matches!(request.mode, MemoryEvictMode::PurgeSession) {
            return self.execute_purge(request).await;
        }

        let session_id = single_session_id(&request.scope)?;
        let tenant_id = resolve_tenant_id(&request.scope, &session_id);
        let max_nodes = clamp_nodes(if request.max_nodes == 0 {
            5000
        } else {
            request.max_nodes
        });

        let candidates = self
            .resolve_candidates(request, &session_id, &tenant_id, max_nodes)
            .await?;

        if candidates.is_empty() {
            return Ok(MemoryEvictResult::default());
        }

        let session_nodes = self
            .store
            .query_nodes_async(NodeQuery {
                limit: TENANT_SCAN_LIMIT,
                session_id: Some(session_id.clone()),
                from_utc: request.scope.from_utc,
                to_utc: request.scope.to_utc,
                tiers: request.scope.tiers.clone(),
            })
            .await?;

        let candidate_sync_keys = candidates
            .iter()
            .map(|candidate| candidate.sync_key.clone())
            .collect::<HashSet<_>>();

        let mut records = Vec::new();
        let mut to_delete_sync_keys = Vec::new();
        let mut to_delete_node_ids = Vec::new();

        for candidate in candidates {
            if !request.force {
                let inbound = collect_inbound_refs(&candidate, &session_nodes, &candidate_sync_keys);
                if !inbound.child_parent_links.is_empty()
                    || !inbound.incoming_semantic_refs.is_empty()
                {
                    records.push(MemoryEvictRecord {
                        node_id: candidate.store_node_id.clone(),
                        sync_key: candidate.sync_key.clone(),
                        status: "blocked".to_string(),
                        reason: Some("inbound references exist".to_string()),
                        inbound_references: Some(InboundReferencesPreview {
                            child_parent_links: inbound.child_parent_links,
                            incoming_semantic_refs: inbound.incoming_semantic_refs,
                        }),
                    });
                    continue;
                }
            }

            if request.dry_run {
                records.push(MemoryEvictRecord {
                    node_id: candidate.store_node_id.clone(),
                    sync_key: candidate.sync_key.clone(),
                    status: "would_delete".to_string(),
                    reason: None,
                    inbound_references: None,
                });
            } else {
                if !candidate.store_node_id.is_empty() {
                    to_delete_node_ids.push(candidate.store_node_id.clone());
                }
                if !candidate.sync_key.is_empty() {
                    to_delete_sync_keys.push(candidate.sync_key.clone());
                }
            }
        }

        let mut core_result = NodeDeleteResult::default();
        if !request.dry_run && (!to_delete_sync_keys.is_empty() || !to_delete_node_ids.is_empty()) {
            let batch_size = clamp_batch_size(500);
            for chunk in to_delete_sync_keys.chunks(batch_size) {
                let chunk_result = self
                    .store
                    .delete_nodes_async(NodeDeleteRequest {
                        tenant_id: tenant_id.clone(),
                        session_id: session_id.clone(),
                        sync_keys: chunk.to_vec(),
                        node_ids: Vec::new(),
                        dry_run: false,
                    })
                    .await?;
                merge_delete_result(&mut core_result, chunk_result);
            }

            for chunk in to_delete_node_ids.chunks(batch_size) {
                let chunk_result = self
                    .store
                    .delete_nodes_async(NodeDeleteRequest {
                        tenant_id: tenant_id.clone(),
                        session_id: session_id.clone(),
                        sync_keys: Vec::new(),
                        node_ids: chunk.to_vec(),
                        dry_run: false,
                    })
                    .await?;
                merge_delete_result(&mut core_result, chunk_result);
            }

            for record in &core_result.records {
                if record.status == NodeDeleteStatus::Deleted && !record.sync_key.is_empty() {
                    self.delete_tag_index_rows(&tenant_id, &record.sync_key).await?;
                }
            }

            for record in core_result.records.iter().filter(|record| {
                record.status == NodeDeleteStatus::Deleted
                    || record.status == NodeDeleteStatus::NotFound
            }) {
                records.push(map_core_record(record));
            }
        }

        let blocked = records.iter().filter(|record| record.status == "blocked").count();
        let would_delete = records
            .iter()
            .filter(|record| record.status == "would_delete")
            .map(|record| record.sync_key.clone())
            .collect::<Vec<_>>();
        let deleted = if request.dry_run {
            would_delete.len()
        } else {
            core_result.deleted
        };
        let not_found = records
            .iter()
            .filter(|record| record.status == "not_found")
            .count()
            + if request.dry_run {
                0
            } else {
                core_result.not_found
            };

        Ok(MemoryEvictResult {
            dry_run: request.dry_run,
            deleted,
            blocked,
            not_found,
            skipped: core_result.skipped,
            would_delete,
            calibrations_deleted: 0,
            checkpoints_deleted: 0,
            records,
        })
    }

    async fn execute_purge(&self, request: &MemoryEvictRequest) -> Result<MemoryEvictResult> {
        let session_id = single_session_id(&request.scope)?;
        let tenant_id = resolve_tenant_id(&request.scope, &session_id);

        let core_result = self
            .store
            .purge_session_async(SessionPurgeRequest {
                tenant_id: tenant_id.clone(),
                session_id: session_id.clone(),
                tiers: request.scope.tiers.clone(),
                dry_run: request.dry_run,
                include_calibration: request.include_calibration,
                include_checkpoints: request.include_checkpoints,
            })
            .await?;

        let records = core_result
            .records
            .iter()
            .map(map_core_record)
            .collect::<Vec<_>>();
        let would_delete = records
            .iter()
            .filter(|record| record.status == "would_delete" || record.status == "skipped")
            .map(|record| record.sync_key.clone())
            .filter(|sync_key| !sync_key.is_empty())
            .collect::<Vec<_>>();

        Ok(MemoryEvictResult {
            dry_run: core_result.dry_run,
            deleted: core_result.deleted,
            blocked: core_result.blocked,
            not_found: core_result.not_found,
            skipped: core_result.skipped,
            would_delete,
            calibrations_deleted: core_result.calibrations_deleted,
            checkpoints_deleted: core_result.checkpoints_deleted,
            records,
        })
    }

    async fn resolve_candidates(
        &self,
        request: &MemoryEvictRequest,
        session_id: &str,
        tenant_id: &str,
        max_nodes: usize,
    ) -> Result<Vec<EvictCandidate>> {
        match request.mode {
            MemoryEvictMode::BySyncKeys => {
                let keys = request
                    .sync_keys
                    .as_ref()
                    .ok_or_else(|| anyhow!("sync_keys are required for by_sync_keys mode"))?;
                if keys.is_empty() {
                    return Err(anyhow!("at least one sync key is required"));
                }

                let session_nodes = self.load_session_nodes(request, session_id).await?;
                let mut candidates = Vec::new();
                for sync_key in keys.iter().map(|value| value.trim()).filter(|value| !value.is_empty()) {
                    if let Some(node) = session_nodes.iter().find(|node| node.sync_key == sync_key) {
                        candidates.push(build_candidate(node, String::new()));
                    } else {
                        candidates.push(EvictCandidate {
                            store_node_id: String::new(),
                            sync_key: sync_key.to_string(),
                            graph_id: String::new(),
                        });
                    }
                }
                Ok(candidates)
            }
            MemoryEvictMode::ByNodeIds => {
                let node_ids = request
                    .node_ids
                    .as_ref()
                    .ok_or_else(|| anyhow!("node_ids are required for by_node_ids mode"))?;
                if node_ids.is_empty() {
                    return Err(anyhow!("at least one node id is required"));
                }

                Ok(node_ids
                    .iter()
                    .map(|node_id| EvictCandidate {
                        store_node_id: node_id.trim().to_string(),
                        sync_key: String::new(),
                        graph_id: String::new(),
                    })
                    .filter(|candidate| !candidate.store_node_id.is_empty())
                    .collect())
            }
            MemoryEvictMode::ByFilter => {
                let nodes = self
                    .load_filtered_nodes(request, session_id, tenant_id, max_nodes)
                    .await?;
                Ok(nodes
                    .iter()
                    .map(|node| build_candidate(node, String::new()))
                    .collect())
            }
            MemoryEvictMode::PurgeSession => Err(anyhow!("purge session uses dedicated path")),
        }
    }

    async fn load_session_nodes(
        &self,
        request: &MemoryEvictRequest,
        session_id: &str,
    ) -> Result<Vec<SttpNode>> {
        self.store
            .query_nodes_async(NodeQuery {
                limit: TENANT_SCAN_LIMIT,
                session_id: Some(session_id.to_string()),
                from_utc: request.scope.from_utc,
                to_utc: request.scope.to_utc,
                tiers: request.scope.tiers.clone(),
            })
            .await
    }

    async fn load_filtered_nodes(
        &self,
        request: &MemoryEvictRequest,
        session_id: &str,
        tenant_id: &str,
        max_nodes: usize,
    ) -> Result<Vec<SttpNode>> {
        let mut nodes = self.load_session_nodes(request, session_id).await?;
        let session_filter = build_session_filter(&request.scope);
        let indexed_sync_keys = if let Some(index) = self.semantic_index.as_ref() {
            resolve_indexed_sync_keys(
                index.as_ref(),
                tenant_id,
                &request.filter,
                Some(session_id),
                TENANT_SCAN_LIMIT,
            )
            .await?
        } else {
            None
        };

        nodes.retain(|node| {
            if let Some(keys) = &indexed_sync_keys
                && !keys.contains(&node.sync_key)
            {
                return false;
            }

            node_matches_common_filters(
                node,
                &request.scope,
                &request.filter,
                session_filter.as_ref(),
            )
        });
        nodes.truncate(max_nodes);
        Ok(nodes)
    }

    async fn delete_tag_index_rows(&self, tenant_id: &str, sync_key: &str) -> Result<()> {
        if let Some(index) = self.semantic_index.as_ref() {
            index
                .delete_node_tags_async(tenant_id, sync_key)
                .await?;
        }
        Ok(())
    }
}

fn single_session_id(scope: &crate::domain::memory::MemoryScope) -> Result<String> {
    scope
        .session_ids
        .as_ref()
        .and_then(|sessions| sessions.first().cloned())
        .filter(|session| !session.trim().is_empty())
        .ok_or_else(|| anyhow!("exactly one session id is required"))
}

fn resolve_tenant_id(
    scope: &crate::domain::memory::MemoryScope,
    session_id: &str,
) -> String {
    scope
        .tenant_id
        .clone()
        .or_else(|| Some(derive_tenant_id_from_session(session_id)))
        .unwrap_or_else(|| "default".to_string())
}

fn build_candidate(node: &SttpNode, store_node_id: String) -> EvictCandidate {
    EvictCandidate {
        store_node_id,
        sync_key: node.sync_key.clone(),
        graph_id: graph_node_id(node),
    }
}

fn collect_inbound_refs(
    candidate: &EvictCandidate,
    nodes: &[SttpNode],
    candidate_sync_keys: &HashSet<String>,
) -> locus_core_rs::domain::models::InboundNodeReferences {
    let mut child_parent_links = BTreeSet::new();
    let mut incoming_semantic_refs = BTreeSet::new();

    for node in nodes {
        if candidate_sync_keys.contains(&node.sync_key) {
            continue;
        }

        if let Some(parent) = node.parent_node_id.as_ref() {
            if parent == &candidate.graph_id
                || (!candidate.store_node_id.is_empty() && parent == &candidate.store_node_id)
            {
                child_parent_links.insert(node.sync_key.clone());
            }
        }

        if let Some(links) = &node.semantic_links {
            for link in links {
                let target = link.target.trim();
                if target_matches_candidate(target, candidate) {
                    incoming_semantic_refs.insert(node.sync_key.clone());
                }
            }
        }
    }

    locus_core_rs::domain::models::InboundNodeReferences {
        child_parent_links: child_parent_links.into_iter().collect(),
        incoming_semantic_refs: incoming_semantic_refs.into_iter().collect(),
    }
}

fn target_matches_candidate(target: &str, candidate: &EvictCandidate) -> bool {
    let lower = target.to_ascii_lowercase();
    if !candidate.sync_key.is_empty()
        && lower == format!("ref:{}", candidate.sync_key.to_ascii_lowercase())
    {
        return true;
    }
    if !candidate.graph_id.is_empty()
        && lower == format!("ref:{}", candidate.graph_id.to_ascii_lowercase())
    {
        return true;
    }
    if !candidate.store_node_id.is_empty()
        && lower == format!("ref:{}", candidate.store_node_id.to_ascii_lowercase())
    {
        return true;
    }
    false
}

fn merge_delete_result(target: &mut NodeDeleteResult, source: NodeDeleteResult) {
    target.deleted += source.deleted;
    target.blocked += source.blocked;
    target.not_found += source.not_found;
    target.skipped += source.skipped;
    target.records.extend(source.records);
}

fn map_core_record(record: &NodeDeleteRecord) -> MemoryEvictRecord {
    let status = match record.status {
        NodeDeleteStatus::Deleted => "deleted",
        NodeDeleteStatus::NotFound => "not_found",
        NodeDeleteStatus::Blocked => "blocked",
        NodeDeleteStatus::Skipped => {
            if record.reason.as_deref() == Some("would delete") {
                "would_delete"
            } else {
                "skipped"
            }
        }
    };

    MemoryEvictRecord {
        node_id: record.node_id.clone(),
        sync_key: record.sync_key.clone(),
        status: status.to_string(),
        reason: record.reason.clone(),
        inbound_references: None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore};
    use locus_core_rs::domain::models::{AvecState, NodeUpsertStatus, SemanticLink, SttpNode};
    use locus_core_rs::storage::{InMemoryNodeStore, InMemorySemanticIndexStore};
    use locus_core_rs::SemanticIndexStoreInitializer;

    use super::*;
    use crate::domain::memory::{MemoryFilter, MemoryScope};

    fn build_node(session_id: &str, sync_key: &str, parent: Option<&str>) -> SttpNode {
        SttpNode {
            raw: "raw".to_string(),
            session_id: session_id.to_string(),
            tier: "raw".to_string(),
            timestamp: Utc::now(),
            compression_depth: 1,
            parent_node_id: parent.map(str::to_string),
            sync_key: sync_key.to_string(),
            updated_at: Utc::now(),
            source_metadata: None,
            context_summary: None,
            semantic_tags: None,
            semantic_links: None,
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedded_at: None,
            user_avec: AvecState::zero(),
            model_avec: AvecState::zero(),
            compression_avec: None,
            rho: 0.9,
            kappa: 0.9,
            psi: 2.0,
        }
    }

    async fn seed_parent_child(store: &InMemoryNodeStore, session: &str) -> (String, String) {
        let parent = build_node(session, "parent-sync", None);
        let parent_graph = graph_node_id(&parent);
        let parent_result = store
            .upsert_node_async(parent)
            .await
            .expect("parent upsert");
        assert_eq!(parent_result.status, NodeUpsertStatus::Created);

        let child = build_node(session, "child-sync", Some(&parent_graph));
        store.upsert_node_async(child).await.expect("child upsert");
        (parent_result.node_id, parent_result.sync_key)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocked_when_child_parent_link_exists() {
        let store = Arc::new(InMemoryNodeStore::new());
        let session = "demo";
        let (_parent_id, parent_sync) = seed_parent_child(store.as_ref(), session).await;

        let service = MemoryEvictService::new(store.clone());
        let result = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::BySyncKeys,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                sync_keys: Some(vec![parent_sync.clone()]),
                dry_run: false,
                force: false,
                ..Default::default()
            })
            .await
            .expect("evict");

        assert_eq!(result.blocked, 1);
        assert_eq!(result.deleted, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn force_deletes_despite_lineage() {
        let store = Arc::new(InMemoryNodeStore::new());
        let session = "demo";
        let (_parent_id, parent_sync) = seed_parent_child(store.as_ref(), session).await;

        let service = MemoryEvictService::new(store.clone());
        let result = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::BySyncKeys,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                sync_keys: Some(vec![parent_sync]),
                dry_run: false,
                force: true,
                ..Default::default()
            })
            .await
            .expect("evict");

        assert_eq!(result.deleted, 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dry_run_does_not_mutate_store() {
        let store = Arc::new(InMemoryNodeStore::new());
        let session = "demo";
        store
            .upsert_node_async(build_node(session, "sync-42", None))
            .await
            .expect("upsert");

        let service = MemoryEvictService::new(store.clone());
        let result = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::BySyncKeys,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                sync_keys: Some(vec!["sync-42".to_string()]),
                dry_run: true,
                ..Default::default()
            })
            .await
            .expect("evict");

        assert_eq!(result.deleted, 1);
        assert_eq!(result.would_delete, vec!["sync-42".to_string()]);

        let remaining = store
            .query_nodes_async(NodeQuery {
                limit: 10,
                session_id: Some(session.to_string()),
                ..Default::default()
            })
            .await
            .expect("query");
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocked_by_inbound_semantic_ref() {
        let store = Arc::new(InMemoryNodeStore::new());
        let session = "demo";
        let target = build_node(session, "target-sync", None);
        let target_graph = graph_node_id(&target);
        store.upsert_node_async(target).await.expect("target");

        let mut referrer = build_node(session, "referrer-sync", None);
        referrer.semantic_links = Some(vec![SemanticLink {
            rel: "evaluates".to_string(),
            target: format!("ref:{target_graph}"),
            confidence: Some(0.9),
        }]);
        store.upsert_node_async(referrer).await.expect("referrer");

        let service = MemoryEvictService::new(store.clone());
        let result = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::BySyncKeys,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                sync_keys: Some(vec!["target-sync".to_string()]),
                dry_run: false,
                force: false,
                ..Default::default()
            })
            .await
            .expect("evict");

        assert_eq!(result.blocked, 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn delete_removes_semantic_tag_index_rows() {
        let store = Arc::new(InMemoryNodeStore::new());
        let index = Arc::new(InMemorySemanticIndexStore::new());
        index.initialize_async().await.expect("init");
        let session = "demo";
        let mut node = build_node(session, "tagged-sync", None);
        node.semantic_tags = Some(vec!["stale".to_string(), "debug".to_string()]);
        store.upsert_node_async(node).await.expect("upsert");

        index
            .sync_node_tags_async(
                locus_core_rs::domain::models::SemanticTagNodeRef {
                    tenant_id: "default".to_string(),
                    session_id: session.to_string(),
                    node_id: "node".to_string(),
                    sync_key: "tagged-sync".to_string(),
                },
                &["stale".to_string(), "debug".to_string()],
                None,
            )
            .await
            .expect("sync tags");

        let service = MemoryEvictService::new(store.clone()).with_semantic_index(index.clone());
        let result = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::BySyncKeys,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                sync_keys: Some(vec!["tagged-sync".to_string()]),
                dry_run: false,
                force: true,
                ..Default::default()
            })
            .await
            .expect("evict");

        assert_eq!(result.deleted, 1);

        let tags = index
            .query_tag_records_async(locus_core_rs::domain::models::SemanticTagQueryFilter {
                tenant_id: Some("default".to_string()),
                session_id: Some(session.to_string()),
                ..Default::default()
            })
            .await
            .expect("query tags");
        assert!(tags.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn session_purge_removes_nodes_and_checkpoints() {
        let store = Arc::new(InMemoryNodeStore::new());
        let session = "temp-import";
        store
            .upsert_node_async(build_node(session, "one", None))
            .await
            .expect("upsert");
        store
            .upsert_node_async(build_node(session, "two", None))
            .await
            .expect("upsert");
        store
            .put_checkpoint_async(locus_core_rs::domain::models::SyncCheckpoint {
                session_id: session.to_string(),
                connector_id: "demo".to_string(),
                cursor: None,
                metadata: None,
                updated_at: Utc::now(),
            })
            .await
            .expect("checkpoint");

        let service = MemoryEvictService::new(store.clone());
        let preview = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::PurgeSession,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                dry_run: true,
                include_checkpoints: true,
                ..Default::default()
            })
            .await
            .expect("preview");
        assert_eq!(preview.deleted, 2);
        assert_eq!(preview.checkpoints_deleted, 1);

        let applied = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::PurgeSession,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                include_checkpoints: true,
                include_calibration: true,
                ..Default::default()
            })
            .await
            .expect("purge");
        assert_eq!(applied.deleted, 2);

        let remaining = store
            .query_nodes_async(NodeQuery {
                limit: 10,
                session_id: Some(session.to_string()),
                ..Default::default()
            })
            .await
            .expect("query");
        assert!(remaining.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn filter_mode_evicts_matching_nodes() {
        let store = Arc::new(InMemoryNodeStore::new());
        let session = "demo";
        let mut stale = build_node(session, "stale-node", None);
        stale.semantic_tags = Some(vec!["stale".to_string()]);
        store.upsert_node_async(stale).await.expect("stale");
        store
            .upsert_node_async(build_node(session, "fresh-node", None))
            .await
            .expect("fresh");

        let service = MemoryEvictService::new(store.clone());
        let result = service
            .execute(&MemoryEvictRequest {
                mode: MemoryEvictMode::ByFilter,
                scope: MemoryScope {
                    session_ids: Some(vec![session.to_string()]),
                    ..Default::default()
                },
                filter: MemoryFilter {
                    tags_contains: Some(vec!["stale".to_string()]),
                    ..Default::default()
                },
                force: true,
                ..Default::default()
            })
            .await
            .expect("evict");

        assert_eq!(result.deleted, 1);
        let remaining = store
            .query_nodes_async(NodeQuery {
                limit: 10,
                session_id: Some(session.to_string()),
                ..Default::default()
            })
            .await
            .expect("query");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].sync_key, "fresh-node");
    }
}
