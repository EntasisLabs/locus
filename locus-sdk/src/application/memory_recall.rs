use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::ContextQueryService;
use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore};
use locus_core_rs::domain::models::{AvecState, PsiRange, SemanticTagQueryFilter, SttpNode};
use locus_core_rs::storage::derive_tenant_id_from_session;

use crate::application::memory_filters::{
    build_session_filter, node_matches_common_filters, resolve_indexed_sync_keys,
};
use crate::domain::memory::{
    FallbackPolicy, MemoryRecallRequest, MemoryRecallResult, RetrievalPath, clamp_limit,
};

pub struct MemoryRecallService {
    context_query: ContextQueryService,
    semantic_index: Option<Arc<dyn SemanticIndexStore>>,
}

impl MemoryRecallService {
    /// Create a recall service backed by the core resonance query pipeline.
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self {
            context_query: ContextQueryService::new(store),
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
            let need_fallback = match request.scoring.fallback_policy {
                FallbackPolicy::Never => false,
                FallbackPolicy::OnEmpty => nodes.is_empty(),
                FallbackPolicy::Always => true,
            };

            if need_fallback {
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

                let lexical = lexical_filter(
                    filter_nodes(
                        fallback_result.nodes,
                        request,
                        session_filter.as_ref(),
                        indexed_sync_keys.as_ref(),
                    ),
                    query_text,
                );

                if request.scoring.fallback_policy == FallbackPolicy::Always && !nodes.is_empty() {
                    nodes = merge_unique(nodes, lexical);
                } else {
                    nodes = lexical;
                }

                path = RetrievalPath::LexicalFallback;
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

    scores.sort_by(|left, right| right.1.partial_cmp(&left.1).unwrap_or(std::cmp::Ordering::Equal));

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
    nodes.into_iter()
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

fn merge_unique(primary: Vec<SttpNode>, secondary: Vec<SttpNode>) -> Vec<SttpNode> {
    let mut merged = Vec::with_capacity(primary.len() + secondary.len());
    let mut seen = HashSet::new();

    for node in primary.into_iter().chain(secondary.into_iter()) {
        if seen.insert(node.sync_key.clone()) {
            merged.push(node);
        }
    }

    merged
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
