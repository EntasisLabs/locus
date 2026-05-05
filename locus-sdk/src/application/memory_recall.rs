use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use locus_core::ContextQueryService;
use locus_core::domain::contracts::NodeStore;
use locus_core::domain::models::{AvecState, PsiRange, SttpNode};

use crate::application::memory_filters::{build_session_filter, node_matches_common_filters};
use crate::domain::memory::{
    FallbackPolicy, MemoryRecallRequest, MemoryRecallResult, RetrievalPath, clamp_limit,
};

pub struct MemoryRecallService {
    context_query: ContextQueryService,
}

impl MemoryRecallService {
    /// Create a recall service backed by the core resonance query pipeline.
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self {
            context_query: ContextQueryService::new(store),
        }
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
                    request.scope.from_utc,
                    request.scope.to_utc,
                    request.scope.tiers.as_deref(),
                    limit,
                )
                .await
        };

        let mut nodes = filter_nodes(primary.nodes, request, session_filter.as_ref());

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
                    filter_nodes(fallback_result.nodes, request, session_filter.as_ref()),
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

fn filter_nodes(
    nodes: Vec<SttpNode>,
    request: &MemoryRecallRequest,
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
