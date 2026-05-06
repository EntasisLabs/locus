use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::domain::contracts::NodeStore;
use locus_core_rs::domain::models::{AvecState, NodeQuery, SttpNode};

use crate::application::memory_filters::{build_session_filter, node_matches_common_filters};
use crate::domain::memory::{
    MemoryAggregateGroup, MemoryAggregateRequest, MemoryAggregateResult, MemoryGroupBy, NumericStats,
    clamp_groups, clamp_nodes,
};

pub struct MemoryAggregateService {
    store: Arc<dyn NodeStore>,
}

impl MemoryAggregateService {
    /// Create an aggregate service over a shared node store.
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self { store }
    }

    /// Compute grouped aggregate statistics over filtered memory nodes.
    ///
    /// Returns group-level coverage and AVEC/metric summaries,
    /// capped by request node/group limits.
    pub async fn execute(&self, request: &MemoryAggregateRequest) -> Result<MemoryAggregateResult> {
        let max_nodes = clamp_nodes(if request.max_nodes == 0 {
            5000
        } else {
            request.max_nodes
        });
        let max_groups = clamp_groups(if request.max_groups == 0 {
            500
        } else {
            request.max_groups
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

        let filtered = nodes
            .into_iter()
            .filter(|node| {
                node_matches_common_filters(node, &request.scope, &request.filter, session_filter.as_ref())
            })
            .collect::<Vec<_>>();

        let scanned_nodes = filtered.len();

        let mut grouped: HashMap<String, Vec<SttpNode>> = HashMap::new();
        for node in filtered {
            let key = group_key(&node, request.group_by);
            grouped.entry(key).or_default().push(node);
        }

        let mut groups = grouped
            .into_iter()
            .map(|(key, nodes)| to_group(key, &nodes))
            .collect::<Vec<_>>();

        groups.sort_by(|left, right| {
            right
                .node_count
                .cmp(&left.node_count)
                .then_with(|| left.key.cmp(&right.key))
        });

        let total_groups = groups.len();
        groups.truncate(max_groups);

        Ok(MemoryAggregateResult {
            groups,
            total_groups,
            scanned_nodes,
        })
    }
}

fn group_key(node: &SttpNode, group_by: MemoryGroupBy) -> String {
    match group_by {
        MemoryGroupBy::SessionId => node.session_id.clone(),
        MemoryGroupBy::Tier => node.tier.clone(),
        MemoryGroupBy::EmbeddingModel => node
            .embedding_model
            .clone()
            .unwrap_or_else(|| "none".to_string()),
        MemoryGroupBy::DateDay => node.timestamp.date_naive().to_string(),
    }
}

fn to_group(key: String, nodes: &[SttpNode]) -> MemoryAggregateGroup {
    let node_count = nodes.len();

    let embedding_count = nodes
        .iter()
        .filter(|node| node.embedding.as_ref().is_some_and(|values| !values.is_empty()))
        .count();

    let embedding_coverage = if node_count == 0 {
        0.0
    } else {
        embedding_count as f32 / node_count as f32
    };

    let avg_user_avec = average_avec(nodes.iter().map(|node| node.user_avec).collect::<Vec<_>>().as_slice());
    let avg_model_avec =
        average_avec(nodes.iter().map(|node| node.model_avec).collect::<Vec<_>>().as_slice());

    let compression_states = nodes
        .iter()
        .filter_map(|node| node.compression_avec)
        .collect::<Vec<_>>();

    let avg_compression_avec = if compression_states.is_empty() {
        None
    } else {
        Some(average_avec(compression_states.as_slice()))
    };

    let psi_stats = average_metric(nodes.iter().map(|node| node.psi).collect::<Vec<_>>().as_slice());
    let rho_stats = average_metric(nodes.iter().map(|node| node.rho).collect::<Vec<_>>().as_slice());
    let kappa_stats =
        average_metric(nodes.iter().map(|node| node.kappa).collect::<Vec<_>>().as_slice());

    MemoryAggregateGroup {
        key,
        node_count,
        embedding_coverage,
        avg_user_avec,
        avg_model_avec,
        avg_compression_avec,
        psi_stats,
        rho_stats,
        kappa_stats,
    }
}

fn average_avec(values: &[AvecState]) -> AvecState {
    if values.is_empty() {
        return AvecState::zero();
    }

    let mut stability = 0.0_f32;
    let mut friction = 0.0_f32;
    let mut logic = 0.0_f32;
    let mut autonomy = 0.0_f32;

    for value in values {
        stability += value.stability;
        friction += value.friction;
        logic += value.logic;
        autonomy += value.autonomy;
    }

    let count = values.len() as f32;

    AvecState {
        stability: stability / count,
        friction: friction / count,
        logic: logic / count,
        autonomy: autonomy / count,
    }
}

fn average_metric(values: &[f32]) -> NumericStats {
    if values.is_empty() {
        return NumericStats::default();
    }

    let (min, max, sum) = values.iter().fold(
        (f32::MAX, f32::MIN, 0.0_f32),
        |(min, max, sum), value| (min.min(*value), max.max(*value), sum + *value),
    );

    NumericStats {
        min,
        max,
        average: sum / values.len() as f32,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use locus_core_rs::{InMemoryNodeStore, NodeStore};
    use locus_core_rs::domain::models::{AvecState, SttpNode};

    use super::MemoryAggregateService;
    use crate::domain::memory::{MemoryAggregateRequest, MemoryGroupBy};

    #[tokio::test]
    async fn aggregates_nodes_by_session_with_coverage() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let now = Utc::now();

        store
            .upsert_node_async(test_node("s-1", "raw", now - Duration::minutes(2), Some(vec![0.1, 0.2])))
            .await
            .expect("upsert should succeed");
        store
            .upsert_node_async(test_node("s-1", "raw", now - Duration::minutes(1), None))
            .await
            .expect("upsert should succeed");
        store
            .upsert_node_async(test_node("s-2", "raw", now, Some(vec![0.3, 0.4])))
            .await
            .expect("upsert should succeed");

        let service = MemoryAggregateService::new(store);
        let request = MemoryAggregateRequest {
            group_by: MemoryGroupBy::SessionId,
            max_groups: 10,
            max_nodes: 100,
            ..Default::default()
        };

        let result = service.execute(&request).await.expect("aggregate should succeed");

        assert_eq!(result.total_groups, 2);
        let s1 = result
            .groups
            .iter()
            .find(|group| group.key == "s-1")
            .expect("s-1 group should exist");
        assert_eq!(s1.node_count, 2);
        assert!((s1.embedding_coverage - 0.5).abs() < f32::EPSILON);
    }

    fn test_node(session_id: &str, tier: &str, timestamp: chrono::DateTime<Utc>, embedding: Option<Vec<f32>>) -> SttpNode {
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
            raw: format!("raw:{session_id}:{tier}:{timestamp}"),
            session_id: session_id.to_string(),
            tier: tier.to_string(),
            timestamp,
            compression_depth: 1,
            parent_node_id: None,
            sync_key: format!("{}:{}:{}", session_id, tier, timestamp.timestamp_nanos_opt().unwrap_or_default()),
            updated_at: timestamp,
            source_metadata: None,
            context_summary: Some("summary".to_string()),
            embedding_dimensions: embedding.as_ref().map(|v| v.len()),
            embedding_model: embedding.as_ref().map(|_| "test-model".to_string()),
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
