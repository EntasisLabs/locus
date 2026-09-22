use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::ContextQueryService;
use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore};
use locus_core_rs::domain::models::{
    AvecState, NodeQuery, PsiRange, SemanticTagQueryFilter, SttpNode,
};
use locus_core_rs::storage::derive_tenant_id_from_session;

use crate::application::memory_filters::{
    build_session_filter, node_matches_common_filters, resolve_indexed_sync_keys,
};
use crate::application::memory_lexical::{
    self, LEXICAL_SCAN_LIMIT, LexicalActivation, LexicalFields,
};
use crate::domain::memory::{
    FallbackPolicy, MemoryRecallRequest, MemoryRecallResult, RetrievalPath, clamp_limit,
};

pub struct MemoryRecallService {
    store: Arc<dyn NodeStore>,
    context_query: ContextQueryService,
    semantic_index: Option<Arc<dyn SemanticIndexStore>>,
}

impl MemoryRecallService {
    /// Create a recall service backed by the core resonance query pipeline.
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self {
            context_query: ContextQueryService::new(store.clone()),
            store,
            semantic_index: None,
        }
    }

    pub fn with_semantic_index(mut self, semantic_index: Arc<dyn SemanticIndexStore>) -> Self {
        self.semantic_index = Some(semantic_index);
        self
    }

    /// Retrieve context nodes using resonance or hybrid scoring,
    /// with optional lexical fallback when configured.
    pub async fn execute(&self, request: &MemoryRecallRequest) -> Result<MemoryRecallResult> {
        let limit = clamp_limit(request.page.limit);
        let expanded_limit = (limit.saturating_mul(5)).clamp(1, 200);

        let current = request.current_avec.unwrap_or_else(AvecState::zero);
        let session_scope = request
            .scope
            .session_ids
            .as_deref()
            .filter(|sessions| sessions.len() == 1)
            .and_then(|sessions| sessions.first().map(String::as_str));
        let session_filter = build_session_filter(&request.scope);
        let tenant_id = request
            .scope
            .tenant_id
            .clone()
            .or_else(|| session_scope.map(derive_tenant_id_from_session))
            .unwrap_or_else(|| "default".to_string());

        let indexed_sync_keys = if let Some(index) = self.semantic_index.as_ref() {
            resolve_indexed_sync_keys(
                index.as_ref(),
                &tenant_id,
                &request.filter,
                session_scope,
                expanded_limit,
            )
            .await?
        } else {
            None
        };

        let mut path = if request.query_embedding.is_some() {
            RetrievalPath::Hybrid
        } else {
            RetrievalPath::ResonanceOnly
        };

        let primary = if let Some(query_embedding) = request.query_embedding.as_deref() {
            self.context_query
                .get_context_hybrid_scoped_filtered_async(
                    session_scope,
                    current.stability,
                    current.friction,
                    current.logic,
                    current.autonomy,
                    request.scope.from_utc,
                    request.scope.to_utc,
                    request.scope.tiers.as_deref(),
                    Some(query_embedding),
                    request.scoring.alpha,
                    request.scoring.beta,
                    expanded_limit,
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
                    request.scope.from_utc,
                    request.scope.to_utc,
                    request.scope.tiers.as_deref(),
                    expanded_limit,
                )
                .await
        };

        let mut nodes = filter_nodes(
            primary.nodes,
            request,
            session_filter.as_ref(),
            indexed_sync_keys.as_ref(),
        );

        if let Some(query_text) = request.query_text.as_deref() {
            let primary_empty = nodes.is_empty();
            match memory_lexical::activation(
                request.scoring.fallback_policy,
                query_text,
                primary_empty,
            ) {
                LexicalActivation::Skip => {}
                LexicalActivation::Legacy => {
                    let fallback_result = self
                        .context_query
                        .get_context_scoped_filtered_async(
                            session_scope,
                            current.stability,
                            current.friction,
                            current.logic,
                            current.autonomy,
                            request.scope.from_utc,
                            request.scope.to_utc,
                            request.scope.tiers.as_deref(),
                            expanded_limit,
                        )
                        .await;

                    let lexical = memory_lexical::legacy_phrase_filter(
                        filter_nodes(
                            fallback_result.nodes,
                            request,
                            session_filter.as_ref(),
                            indexed_sync_keys.as_ref(),
                        ),
                        query_text,
                    );

                    if request.scoring.fallback_policy == FallbackPolicy::Always
                        && !nodes.is_empty()
                    {
                        nodes = memory_lexical::merge_unique(nodes, lexical);
                    } else {
                        nodes = lexical;
                    }

                    path = RetrievalPath::LexicalFallback;
                }
                LexicalActivation::NaturalLanguage => {
                    let scanned = self
                        .store
                        .query_nodes_async(NodeQuery {
                            limit: LEXICAL_SCAN_LIMIT,
                            session_id: session_scope.map(str::to_string),
                            from_utc: request.scope.from_utc,
                            to_utc: request.scope.to_utc,
                            tiers: request.scope.tiers.clone(),
                        })
                        .await?;
                    let lexical = memory_lexical::select_lexical_matches(
                        filter_nodes(
                            scanned,
                            request,
                            session_filter.as_ref(),
                            indexed_sync_keys.as_ref(),
                        ),
                        &memory_lexical::parse_lexical_query(query_text),
                        request.scoring.strictness,
                        LexicalFields::RECALL,
                    );
                    let (ranked, applied) = memory_lexical::apply_natural_language(
                        nodes,
                        lexical,
                        request.query_embedding.is_some(),
                    );
                    nodes = ranked;
                    if request.query_embedding.is_none() && (applied || primary_empty) {
                        path = RetrievalPath::LexicalFallback;
                    }
                }
            }
        }

        if request.scoring.gamma > 0.0
            && let Some(query_tag_embedding) = request.query_tag_embedding.as_deref()
            && let Some(index) = self.semantic_index.as_ref()
        {
            rerank_by_tag_similarity(
                &mut nodes,
                index.as_ref(),
                &tenant_id,
                query_tag_embedding,
                request.scoring.gamma,
            )
            .await?;
        }

        let has_more = nodes.len() > limit;
        nodes.truncate(limit);

        let next_cursor = nodes
            .last()
            .map(|node| format!("{}|{}", node.updated_at.to_rfc3339(), node.sync_key));

        let psi_range = psi_range_from_nodes(&nodes);

        Ok(MemoryRecallResult {
            retrieved: nodes.len(),
            nodes,
            psi_range,
            retrieval_path: path,
            has_more,
            next_cursor,
        })
    }
}

async fn rerank_by_tag_similarity(
    nodes: &mut Vec<SttpNode>,
    index: &dyn SemanticIndexStore,
    tenant_id: &str,
    query_embedding: &[f32],
    gamma: f32,
) -> Result<()> {
    if nodes.is_empty() {
        return Ok(());
    }

    let sync_keys: Vec<String> = nodes.iter().map(|node| node.sync_key.clone()).collect();
    let records = index
        .query_tag_records_async(SemanticTagQueryFilter {
            tenant_id: Some(tenant_id.to_string()),
            tags: None,
            tag_prefix: None,
            has_embedding: Some(true),
            missing_embedding_only: false,
            limit: sync_keys.len().saturating_mul(16).max(64),
            session_id: None,
        })
        .await?;

    let mut scores: Vec<(usize, f32)> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let tag_score = records
                .iter()
                .filter(|record| record.sync_key == node.sync_key)
                .filter_map(|record| record.embedding.as_deref())
                .filter_map(|embedding| cosine_similarity(query_embedding, embedding))
                .fold(0.0_f32, f32::max);
            (index, tag_score)
        })
        .collect();

    scores.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut reranked = Vec::with_capacity(nodes.len());
    let mut used = HashSet::new();
    for (index, _) in scores {
        if used.insert(index) {
            reranked.push(nodes[index].clone());
        }
    }

    if gamma >= 1.0 {
        *nodes = reranked;
    } else {
        let blend_count = ((nodes.len() as f32) * gamma).ceil() as usize;
        for (slot, node) in reranked.into_iter().take(blend_count).enumerate() {
            nodes[slot] = node;
        }
    }

    Ok(())
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }

    let mut dot = 0.0_f32;
    let mut left_norm = 0.0_f32;
    let mut right_norm = 0.0_f32;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        return None;
    }

    Some(dot / (left_norm.sqrt() * right_norm.sqrt()))
}

fn filter_nodes(
    nodes: Vec<SttpNode>,
    request: &MemoryRecallRequest,
    session_filter: Option<&HashSet<String>>,
    indexed_sync_keys: Option<&HashSet<String>>,
) -> Vec<SttpNode> {
    nodes
        .into_iter()
        .filter(|node| {
            if let Some(keys) = indexed_sync_keys
                && !keys.contains(&node.sync_key)
            {
                return false;
            }

            node_matches_common_filters(node, &request.scope, &request.filter, session_filter)
        })
        .collect()
}

fn psi_range_from_nodes(nodes: &[SttpNode]) -> PsiRange {
    if nodes.is_empty() {
        return PsiRange::default();
    }

    let (min, max, sum) = nodes
        .iter()
        .fold((f32::MAX, f32::MIN, 0.0_f32), |(min, max, sum), node| {
            (min.min(node.psi), max.max(node.psi), sum + node.psi)
        });

    PsiRange {
        min,
        max,
        average: sum / nodes.len() as f32,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use locus_core_rs::domain::models::{AvecState, SttpNode};
    use locus_core_rs::{InMemoryNodeStore, NodeStore};

    use super::MemoryRecallService;
    use crate::domain::memory::{
        FallbackPolicy, MemoryPage, MemoryRecallRequest, MemoryScoring, RetrievalPath,
    };

    #[tokio::test]
    async fn natural_language_question_returns_the_matching_memory() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        store
            .upsert_node_async(sample(
                "near",
                AvecState::zero(),
                "weekend hiking plan",
                "unrelated notes",
            ))
            .await
            .expect("upsert near node");
        store
            .upsert_node_async(sample(
                "far",
                AvecState {
                    stability: 1.0,
                    friction: 1.0,
                    logic: 0.0,
                    autonomy: 0.0,
                },
                "decided to harden the parser grammar",
                "decision notes",
            ))
            .await
            .expect("upsert far node");

        let service = MemoryRecallService::new(store);
        let result = service
            .execute(&MemoryRecallRequest {
                page: MemoryPage {
                    limit: 1,
                    cursor: None,
                },
                scoring: MemoryScoring {
                    fallback_policy: FallbackPolicy::OnEmpty,
                    ..Default::default()
                },
                current_avec: Some(AvecState::zero()),
                query_text: Some("what did we decide about the parser grammar?".to_string()),
                ..Default::default()
            })
            .await
            .expect("recall should succeed");

        assert_eq!(result.retrieval_path, RetrievalPath::LexicalFallback);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].sync_key, "far");
    }

    #[tokio::test]
    async fn single_token_does_not_override_resonance_when_primary_is_non_empty() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        store
            .upsert_node_async(sample(
                "near",
                AvecState::zero(),
                "weekend plans",
                "alpha notes",
            ))
            .await
            .expect("upsert near node");
        store
            .upsert_node_async(sample(
                "far",
                AvecState {
                    stability: 1.0,
                    friction: 1.0,
                    logic: 0.0,
                    autonomy: 0.0,
                },
                "hiking notes",
                "bring boots",
            ))
            .await
            .expect("upsert far node");

        let service = MemoryRecallService::new(store);
        let result = service
            .execute(&MemoryRecallRequest {
                page: MemoryPage {
                    limit: 1,
                    cursor: None,
                },
                scoring: MemoryScoring {
                    fallback_policy: FallbackPolicy::OnEmpty,
                    ..Default::default()
                },
                current_avec: Some(AvecState::zero()),
                query_text: Some("hiking".to_string()),
                ..Default::default()
            })
            .await
            .expect("recall should succeed");

        assert_eq!(result.retrieval_path, RetrievalPath::ResonanceOnly);
        assert_eq!(result.nodes[0].sync_key, "near");
    }

    fn sample(sync_key: &str, avec: AvecState, summary: &str, raw: &str) -> SttpNode {
        let now = Utc::now();
        SttpNode {
            raw: raw.to_string(),
            session_id: "session".to_string(),
            tier: "raw".to_string(),
            timestamp: now,
            compression_depth: 1,
            parent_node_id: None,
            sync_key: sync_key.to_string(),
            updated_at: now,
            source_metadata: None,
            context_summary: Some(summary.to_string()),
            semantic_tags: None,
            semantic_links: None,
            embedding_dimensions: None,
            embedding_model: None,
            embedding: None,
            embedded_at: None,
            user_avec: avec,
            model_avec: avec,
            compression_avec: Some(avec),
            rho: 0.5,
            kappa: 0.5,
            psi: 1.0,
        }
    }
}
