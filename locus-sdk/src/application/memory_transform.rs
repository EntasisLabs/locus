use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use locus_core::domain::contracts::NodeStore;
use locus_core::domain::models::{NodeQuery, NodeUpsertStatus};

use crate::application::ai_router::route_embedding;
use crate::application::memory_filters::{build_session_filter, node_matches_common_filters};
use crate::domain::ai::{AiProviderRegistry, AiTask, EmbedRequest, ProviderPolicy};
use crate::domain::memory::{
    MemoryTransformOperation, MemoryTransformRequest, MemoryTransformResult, clamp_batch_size,
    clamp_nodes,
};

pub struct MemoryTransformService {
    store: Arc<dyn NodeStore>,
    providers: Arc<dyn AiProviderRegistry>,
}

impl MemoryTransformService {
    /// Create a transform service with storage and provider registry dependencies.
    pub fn new(store: Arc<dyn NodeStore>, providers: Arc<dyn AiProviderRegistry>) -> Self {
        Self { store, providers }
    }

    /// Execute a bulk memory transform operation.
    ///
    /// The current implementation supports embedding backfill with optional
    /// dry-run behavior, batch control, and bounded failure reporting.
    pub async fn execute(&self, request: &MemoryTransformRequest) -> Result<MemoryTransformResult> {
        let started_at = Utc::now();
        let max_nodes = clamp_nodes(if request.max_nodes == 0 {
            5000
        } else {
            request.max_nodes
        });
        let batch_size = clamp_batch_size(if request.batch_size == 0 {
            100
        } else {
            request.batch_size
        });

        let single_session = request
            .scope
            .session_ids
            .as_deref()
            .filter(|sessions| sessions.len() == 1)
            .and_then(|sessions| sessions.first().cloned());

        let nodes = self
            .store
            .query_nodes_async(NodeQuery {
                limit: max_nodes,
                session_id: single_session,
                from_utc: request.scope.from_utc,
                to_utc: request.scope.to_utc,
                tiers: request.scope.tiers.clone(),
            })
            .await?;

        let session_filter = build_session_filter(&request.scope);

        let mut selected = nodes
            .into_iter()
            .filter(|node| {
                node_matches_common_filters(node, &request.scope, &request.filter, session_filter.as_ref())
            })
            .collect::<Vec<_>>();

        if request.operation == MemoryTransformOperation::EmbedBackfill {
            selected.retain(|node| node.embedding.as_ref().is_none_or(|values| values.is_empty()));
        }

        let mut result = MemoryTransformResult {
            scanned: selected.len(),
            selected: selected.len(),
            started_at,
            completed_at: started_at,
            ..Default::default()
        };

        if request.dry_run {
            result.updated = result.selected;
            result.completed_at = Utc::now();
            return Ok(result);
        }

        for chunk in selected.chunks(batch_size) {
            for mut node in chunk.iter().cloned() {
                let Some(embedding_input) = build_embedding_input(node.context_summary.as_deref(), &node.session_id)
                else {
                    result.skipped += 1;
                    continue;
                };

                let embed_request = EmbedRequest {
                    text: embedding_input,
                    task: AiTask::SemanticEmbedding,
                    provider_id: request.provider_id.clone(),
                    model: request.model.clone(),
                    policy: if request.provider_id.is_some() {
                        ProviderPolicy::Required
                    } else {
                        ProviderPolicy::Auto
                    },
                };

                let vector = match route_embedding(self.providers.as_ref(), &embed_request).await {
                    Ok(values) if !values.is_empty() => values,
                    Ok(_) => {
                        result.failed += 1;
                        push_failure(
                            &mut result.failures,
                            format!("{}: embedding provider returned empty vector", node.sync_key),
                        );
                        continue;
                    }
                    Err(err) => {
                        result.failed += 1;
                        push_failure(
                            &mut result.failures,
                            format!("{}: embedding failed: {err}", node.sync_key),
                        );
                        continue;
                    }
                };

                node.embedding_dimensions = Some(vector.len());
                node.embedding_model = request
                    .model
                    .clone()
                    .or_else(|| request.provider_id.clone())
                    .or_else(|| Some("sdk-memory-transform".to_string()));
                node.embedding = Some(vector);
                node.embedded_at = Some(Utc::now());
                node.updated_at = Utc::now();

                match self.store.upsert_node_async(node).await {
                    Ok(status) => match status.status {
                        NodeUpsertStatus::Created | NodeUpsertStatus::Updated => result.updated += 1,
                        NodeUpsertStatus::Duplicate => result.duplicate += 1,
                        NodeUpsertStatus::Skipped => result.skipped += 1,
                    },
                    Err(err) => {
                        result.failed += 1;
                        push_failure(&mut result.failures, format!("store upsert failed: {err}"));
                    }
                }
            }
        }

        result.completed_at = Utc::now();
        Ok(result)
    }
}

fn build_embedding_input(context_summary: Option<&str>, session_id: &str) -> Option<String> {
    let summary = context_summary.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let session = {
        let trimmed = session_id.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    };

    match (summary, session) {
        (Some(summary), Some(session)) => Some(format!("{summary}\nsession_id:{session}")),
        (Some(summary), None) => Some(summary.to_string()),
        (None, Some(session)) => Some(format!("session_id:{session}")),
        (None, None) => None,
    }
}

fn push_failure(failures: &mut Vec<String>, reason: String) {
    if failures.len() < 100 {
        failures.push(reason);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use async_trait::async_trait;
    use chrono::Utc;
    use locus_core::{InMemoryNodeStore, NodeStore};
    use locus_core::domain::models::{AvecState, SttpNode};

    use super::MemoryTransformService;
    use crate::domain::ai::{
        AiCapability, AiProvider, EmbedRequest, ScoreAvecRequest,
    };
    use crate::domain::memory::{MemoryTransformOperation, MemoryTransformRequest};
    use crate::infrastructure::registry::InMemoryAiProviderRegistry;

    struct MockEmbeddingProvider;

    #[async_trait]
    impl AiProvider for MockEmbeddingProvider {
        fn provider_id(&self) -> &str {
            "mock"
        }

        fn capabilities(&self) -> &'static [AiCapability] {
            &[AiCapability::SemanticEmbedding]
        }

        async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![0.2, 0.3, 0.4])
        }

        async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![0.2, 0.3, 0.4])
        }

        async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
            Ok(AvecState::zero())
        }
    }

    #[tokio::test]
    async fn dry_run_reports_selected_without_writes() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let node = test_node("dry-run", None);
        store
            .upsert_node_async(node)
            .await
            .expect("upsert should succeed");

        let mut providers = InMemoryAiProviderRegistry::new();
        providers.register(MockEmbeddingProvider);

        let service = MemoryTransformService::new(store, Arc::new(providers));

        let request = MemoryTransformRequest {
            operation: MemoryTransformOperation::EmbedBackfill,
            dry_run: true,
            max_nodes: 100,
            batch_size: 10,
            ..Default::default()
        };

        let result = service.execute(&request).await.expect("transform should succeed");

        assert_eq!(result.selected, 1);
        assert_eq!(result.updated, 1);
        assert_eq!(result.failed, 0);
    }

    #[tokio::test]
    async fn embed_backfill_updates_missing_embedding_nodes() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let node = test_node("backfill", None);
        store
            .upsert_node_async(node)
            .await
            .expect("upsert should succeed");

        let mut providers = InMemoryAiProviderRegistry::new();
        providers.register(MockEmbeddingProvider);

        let service = MemoryTransformService::new(store.clone(), Arc::new(providers));

        let request = MemoryTransformRequest {
            operation: MemoryTransformOperation::EmbedBackfill,
            dry_run: false,
            max_nodes: 100,
            batch_size: 10,
            ..Default::default()
        };

        let result = service.execute(&request).await.expect("transform should succeed");

        assert_eq!(result.updated, 1);
        assert_eq!(result.failed, 0);

        let nodes = store
            .query_nodes_async(locus_core::domain::models::NodeQuery {
                limit: 10,
                session_id: Some("backfill".to_string()),
                ..Default::default()
            })
            .await
            .expect("query should succeed");

        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].embedding.as_ref().is_some_and(|v| !v.is_empty()));
    }

    fn test_node(session_id: &str, embedding: Option<Vec<f32>>) -> SttpNode {
        let now = Utc::now();
        let user = AvecState {
            stability: 0.6,
            friction: 0.4,
            logic: 0.8,
            autonomy: 0.7,
        };
        let model = AvecState {
            stability: 0.5,
            friction: 0.3,
            logic: 0.9,
            autonomy: 0.6,
        };

        SttpNode {
            raw: format!("raw:{session_id}"),
            session_id: session_id.to_string(),
            tier: "raw".to_string(),
            timestamp: now,
            compression_depth: 1,
            parent_node_id: None,
            sync_key: format!("{}:{}", session_id, now.timestamp_nanos_opt().unwrap_or_default()),
            updated_at: now,
            source_metadata: None,
            context_summary: Some("summary".to_string()),
            embedding_dimensions: embedding.as_ref().map(|v| v.len()),
            embedding_model: embedding.as_ref().map(|_| "existing".to_string()),
            embedding,
            embedded_at: None,
            user_avec: user,
            model_avec: model,
            compression_avec: Some(model),
            rho: 0.9,
            kappa: 0.8,
            psi: 2.5,
        }
    }
}
