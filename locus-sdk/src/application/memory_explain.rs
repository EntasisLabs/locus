use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use locus_core::ContextQueryService;
use locus_core::domain::contracts::NodeStore;
use locus_core::domain::models::{AvecState, SttpNode};

use crate::application::memory_filters::{build_session_filter, node_matches_common_filters};
use crate::domain::memory::{
    FallbackPolicy, MemoryExplainRequest, MemoryExplainResult, MemoryExplainStage, RetrievalPath,
    clamp_limit,
};

pub struct MemoryExplainService {
    context_query: ContextQueryService,
}

impl MemoryExplainService {
    /// Create an explanation service for retrieval-stage introspection.
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self {
            context_query: ContextQueryService::new(store),
        }
    }

    /// Explain retrieval behavior for a recall request.
    ///
    /// Returns per-stage counts, retrieval path, and fallback diagnostics
    /// without mutating stored nodes.
    pub async fn execute(&self, request: &MemoryExplainRequest) -> Result<MemoryExplainResult> {
        let recall = &request.recall;
        let limit = clamp_limit(recall.page.limit);
        let expanded_limit = (limit.saturating_mul(5)).clamp(1, 200);

        let current = recall.current_avec.unwrap_or_else(AvecState::zero);
        let session_scope = recall
            .scope
            .session_ids
            .as_deref()
            .filter(|sessions| sessions.len() == 1)
            .and_then(|sessions| sessions.first().map(String::as_str));
        let session_filter = build_session_filter(&recall.scope);

        let mut stages = Vec::new();
        let mut path = if recall.query_embedding.is_some() {
            RetrievalPath::Hybrid
        } else {
            RetrievalPath::ResonanceOnly
        };
        let mut fallback_triggered = false;
        let mut fallback_reason = None;

        let primary = if let Some(query_embedding) = recall.query_embedding.as_deref() {
            self.context_query
                .get_context_hybrid_scoped_filtered_async(
                    session_scope,
                    current.stability,
                    current.friction,
                    current.logic,
                    current.autonomy,
                    recall.scope.from_utc,
                    recall.scope.to_utc,
                    recall.scope.tiers.as_deref(),
                    Some(query_embedding),
                    recall.scoring.alpha,
                    recall.scoring.beta,
                    limit,
                )
                .await
        } else {
            self.context_query
                .get_context_scoped_filtered_async(
                    session_scope,
                    current.stability,
                    current.friction,
                    current.logic,
                    current.autonomy,
                    recall.scope.from_utc,
                    recall.scope.to_utc,
                    recall.scope.tiers.as_deref(),
                    limit,
                )
                .await
        };

        stages.push(MemoryExplainStage {
            stage: "primary_retrieval".to_string(),
            count: primary.nodes.len(),
        });

        let filtered_primary = filter_nodes(primary.nodes, recall, session_filter.as_ref());
        stages.push(MemoryExplainStage {
            stage: "after_common_filter".to_string(),
            count: filtered_primary.len(),
        });

        if let Some(query_text) = recall.query_text.as_deref() {
            let need_fallback = match recall.scoring.fallback_policy {
                FallbackPolicy::Never => false,
                FallbackPolicy::OnEmpty => filtered_primary.is_empty(),
                FallbackPolicy::Always => true,
            };

            if need_fallback {
                fallback_triggered = true;
                fallback_reason = Some(match recall.scoring.fallback_policy {
                    FallbackPolicy::Never => "never".to_string(),
                    FallbackPolicy::OnEmpty => {
                        "fallback_policy=on_empty and primary result set is empty".to_string()
                    }
                    FallbackPolicy::Always => "fallback_policy=always".to_string(),
                });

                let fallback = self
                    .context_query
                    .get_context_scoped_filtered_async(
                        session_scope,
                        current.stability,
                        current.friction,
                        current.logic,
                        current.autonomy,
                        recall.scope.from_utc,
                        recall.scope.to_utc,
                        recall.scope.tiers.as_deref(),
                        expanded_limit,
                    )
                    .await;

                stages.push(MemoryExplainStage {
                    stage: "fallback_retrieval".to_string(),
                    count: fallback.nodes.len(),
                });

                let filtered_fallback =
                    filter_nodes(fallback.nodes, recall, session_filter.as_ref());
                stages.push(MemoryExplainStage {
                    stage: "fallback_after_common_filter".to_string(),
                    count: filtered_fallback.len(),
                });

                let lexical = lexical_filter(filtered_fallback, query_text);
                stages.push(MemoryExplainStage {
                    stage: "lexical_filter".to_string(),
                    count: lexical.len(),
                });

                path = RetrievalPath::LexicalFallback;
            }
        }

        Ok(MemoryExplainResult {
            retrieval_path: path,
            fallback_triggered,
            fallback_reason,
            stages,
            scoring: recall.scoring.clone(),
        })
    }
}

fn filter_nodes(
    nodes: Vec<SttpNode>,
    request: &crate::domain::memory::MemoryRecallRequest,
    session_filter: Option<&HashSet<String>>,
) -> Vec<SttpNode> {
    nodes.into_iter()
        .filter(|node| {
            node_matches_common_filters(node, &request.scope, &request.filter, session_filter)
        })
        .collect()
}

fn lexical_filter(nodes: Vec<SttpNode>, query_text: &str) -> Vec<SttpNode> {
    let needle = query_text.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return nodes;
    }

    let mut scored = nodes
        .into_iter()
        .filter_map(|node| {
            let summary = node
                .context_summary
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let session = node.session_id.to_ascii_lowercase();
            let raw = node.raw.to_ascii_lowercase();

            let mut score = 0usize;
            if summary.contains(&needle) {
                score += 3;
            }
            if session.contains(&needle) {
                score += 2;
            }
            if raw.contains(&needle) {
                score += 1;
            }

            if score > 0 {
                Some((score, node.timestamp, node))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

    scored.into_iter().map(|(_, _, node)| node).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use locus_core::domain::models::{AvecState, SttpNode};
    use locus_core::{InMemoryNodeStore, NodeStore};

    use super::MemoryExplainService;
    use crate::domain::memory::{
        FallbackPolicy, MemoryExplainRequest, MemoryFilter, MemoryRecallRequest, MemoryScoring,
    };

    #[tokio::test]
    async fn explain_marks_fallback_when_on_empty_and_no_primary_results() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let node = test_node("s-explain", "raw", "some unrelated payload");
        store
            .upsert_node_async(node)
            .await
            .expect("upsert should succeed");

        let service = MemoryExplainService::new(store);
        let request = MemoryExplainRequest {
            recall: MemoryRecallRequest {
                query_text: Some("nonexistent-token".to_string()),
                filter: MemoryFilter {
                    has_embedding: Some(true),
                    ..Default::default()
                },
                scoring: MemoryScoring {
                    fallback_policy: FallbackPolicy::OnEmpty,
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result = service.execute(&request).await.expect("explain should succeed");

        assert!(result.fallback_triggered);
        assert_eq!(result.retrieval_path, crate::domain::memory::RetrievalPath::LexicalFallback);
        assert!(result
            .stages
            .iter()
            .any(|stage| stage.stage == "fallback_retrieval"));
    }

    fn test_node(session_id: &str, tier: &str, raw: &str) -> SttpNode {
        let now = Utc::now();
        let user = AvecState {
            stability: 0.6,
            friction: 0.4,
            logic: 0.8,
            autonomy: 0.7,
        };

        SttpNode {
            raw: raw.to_string(),
            session_id: session_id.to_string(),
            tier: tier.to_string(),
            timestamp: now,
            compression_depth: 1,
            parent_node_id: None,
            sync_key: format!("{session_id}:{tier}:{}", now.timestamp_nanos_opt().unwrap_or_default()),
            updated_at: now,
            source_metadata: None,
            context_summary: Some("summary".to_string()),
            embedding_dimensions: None,
            embedding_model: None,
            embedding: None,
            embedded_at: None,
            user_avec: user,
            model_avec: user,
            compression_avec: Some(user),
            rho: 0.9,
            kappa: 0.8,
            psi: 2.5,
        }
    }
}
