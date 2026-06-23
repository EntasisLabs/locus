use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore};
use locus_core_rs::domain::models::{NodeQuery, SttpNode};
use locus_core_rs::storage::derive_tenant_id_from_session;
use serde_json::json;

use crate::application::memory_filters::{
    build_session_filter, node_matches_common_filters, resolve_indexed_sync_keys,
};
use crate::domain::graph::{MemoryGraphRequest, MemoryGraphResult};
use crate::domain::memory::clamp_limit;

const TENANT_SCAN_LIMIT: usize = 5000;

pub struct MemoryGraphService {
    store: Arc<dyn NodeStore>,
    semantic_index: Option<Arc<dyn SemanticIndexStore>>,
}

impl MemoryGraphService {
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

    pub async fn execute(&self, request: &MemoryGraphRequest) -> Result<MemoryGraphResult> {
        let capped_limit = clamp_limit(if request.limit == 0 {
            1000
        } else {
            request.limit
        })
        .clamp(1, 5000);

        let single_session = request
            .scope
            .session_ids
            .as_deref()
            .filter(|sessions| sessions.len() == 1)
            .and_then(|sessions| sessions.first().cloned());

        let backend_limit = if single_session.is_some() {
            capped_limit
        } else {
            TENANT_SCAN_LIMIT
        };

        let mut nodes = self
            .store
            .query_nodes_async(NodeQuery {
                limit: backend_limit,
                session_id: single_session.clone(),
                from_utc: request.scope.from_utc,
                to_utc: request.scope.to_utc,
                tiers: request.scope.tiers.clone(),
            })
            .await?;

        let session_filter = build_session_filter(&request.scope);
        let tenant_id = request
            .scope
            .tenant_id
            .clone()
            .or_else(|| {
                single_session
                    .as_deref()
                    .map(derive_tenant_id_from_session)
            })
            .unwrap_or_else(|| "default".to_string());

        let indexed_sync_keys = if let Some(index) = self.semantic_index.as_ref() {
            resolve_indexed_sync_keys(
                index.as_ref(),
                &tenant_id,
                &request.filter,
                single_session.as_deref(),
                backend_limit,
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

        nodes.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        nodes.truncate(capped_limit);

        let include_topology = request.include_session_topology;
        let include_lineage = request.include_lineage;
        let include_semantic = request.include_semantic;
        let rel_filter = request
            .rel
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        let target_prefix = request
            .target_prefix
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());

        #[derive(Clone)]
        struct SessionGroup {
            id: String,
            label: String,
            nodes: Vec<SttpNode>,
            node_count: usize,
            avg_psi: f32,
            last_modified: DateTime<Utc>,
            size: usize,
        }

        let mut grouped_map: BTreeMap<String, Vec<SttpNode>> = BTreeMap::new();
        for node in &nodes {
            grouped_map
                .entry(node.session_id.clone())
                .or_default()
                .push(node.clone());
        }

        let mut grouped = grouped_map
            .into_iter()
            .map(|(id, mut session_nodes)| {
                session_nodes.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
                let node_count = session_nodes.len();
                let avg_psi = if node_count == 0 {
                    0.0
                } else {
                    session_nodes.iter().map(|node| node.psi).sum::<f32>() / node_count as f32
                };
                let last_modified = session_nodes
                    .first()
                    .map(|node| node.timestamp)
                    .unwrap_or_else(Utc::now);
                let size = 16 + std::cmp::min(28, node_count * 2);

                SessionGroup {
                    label: id.clone(),
                    id,
                    nodes: session_nodes,
                    node_count,
                    avg_psi,
                    last_modified,
                    size,
                }
            })
            .collect::<Vec<_>>();

        grouped.sort_by(|left, right| right.last_modified.cmp(&left.last_modified));

        let node_by_id = nodes
            .iter()
            .map(|node| (graph_node_id(node), node.clone()))
            .collect::<HashMap<_, _>>();

        let sessions = if include_topology {
            grouped
                .iter()
                .map(|session| {
                    json!({
                        "id": format!("s:{}", session.id),
                        "label": session.label,
                        "nodeCount": session.node_count,
                        "avgPsi": session.avg_psi,
                        "lastModified": session.last_modified.to_rfc3339(),
                        "size": session.size
                    })
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let graph_nodes = nodes
            .iter()
            .map(|node| {
                json!({
                    "id": graph_node_id(node),
                    "sessionId": node.session_id,
                    "label": format!("{} {}", node.tier, node.timestamp.format("%m-%d %H:%M")),
                    "tier": node.tier,
                    "timestamp": node.timestamp.to_rfc3339(),
                    "psi": node.psi,
                    "parentNodeId": node.parent_node_id,
                    "semanticTags": node.semantic_tags,
                    "size": 9
                })
            })
            .collect::<Vec<_>>();

        let mut edges = Vec::new();

        if include_topology {
            for index in 0..grouped.len().saturating_sub(1) {
                edges.push(json!({
                    "id": format!("t-{index}"),
                    "source": format!("s:{}", grouped[index].id),
                    "target": format!("s:{}", grouped[index + 1].id),
                    "kind": "timeline"
                }));
            }

            for index in 0..grouped.len() {
                let from = &grouped[index];
                let mut nearest: Option<usize> = None;
                let mut nearest_distance = f32::MAX;

                for (other_index, other) in grouped.iter().enumerate() {
                    if index == other_index {
                        continue;
                    }
                    let distance = (from.avg_psi - other.avg_psi).abs();
                    if distance < nearest_distance {
                        nearest_distance = distance;
                        nearest = Some(other_index);
                    }
                }

                if let Some(nearest_index) = nearest
                    && index < nearest_index
                {
                    edges.push(json!({
                        "id": format!("s-{index}-{nearest_index}"),
                        "source": format!("s:{}", from.id),
                        "target": format!("s:{}", grouped[nearest_index].id),
                        "kind": "similarity"
                    }));
                }
            }
        }

        for session in &grouped {
            for index in 0..session.nodes.len() {
                let current = &session.nodes[index];
                let current_id = graph_node_id(current);

                if include_topology {
                    edges.push(json!({
                        "id": format!("m-{}-{index}", session.id),
                        "source": format!("s:{}", session.id),
                        "target": current_id,
                        "kind": "membership"
                    }));

                    if index + 1 < session.nodes.len() {
                        let older = &session.nodes[index + 1];
                        edges.push(json!({
                            "id": format!("nt-{}-{index}", session.id),
                            "source": current_id,
                            "target": graph_node_id(older),
                            "kind": "node_timeline"
                        }));
                    }
                }

                if include_lineage
                    && let Some(parent) = current.parent_node_id.as_ref()
                    && node_by_id.contains_key(parent)
                {
                    edges.push(json!({
                        "id": format!("l-{}-{index}", session.id),
                        "source": current_id,
                        "target": parent,
                        "kind": "lineage"
                    }));
                }

                if include_semantic
                    && let Some(links) = current.semantic_links.as_ref()
                {
                    for (link_index, link) in links.iter().enumerate() {
                        if let Some(expected_rel) = rel_filter.as_deref()
                            && link.rel.trim().to_ascii_lowercase() != expected_rel
                        {
                            continue;
                        }

                        if let Some(prefix) = target_prefix.as_deref()
                            && !link.target.trim().to_ascii_lowercase().starts_with(prefix)
                        {
                            continue;
                        }

                        let target = if let Some(reference) = link.target.strip_prefix("ref:") {
                            let reference = reference.trim();
                            if node_by_id.contains_key(reference) {
                                reference.to_string()
                            } else {
                                link.target.clone()
                            }
                        } else {
                            link.target.clone()
                        };

                        let mut edge = json!({
                            "id": format!("sl-{}-{index}-{link_index}", session.id),
                            "source": current_id,
                            "target": target,
                            "kind": "semantic",
                            "rel": link.rel,
                        });
                        if let Some(confidence) = link.confidence {
                            edge.as_object_mut().map(|object| {
                                object.insert("confidence".to_string(), json!(confidence));
                            });
                        }
                        edges.push(edge);
                    }
                }
            }
        }

        Ok(MemoryGraphResult {
            sessions,
            nodes: graph_nodes,
            edges,
            retrieved: nodes.len(),
        })
    }
}

pub fn graph_node_id(node: &SttpNode) -> String {
    format!(
        "n:{}|{}|{}|{:.4}",
        node.session_id,
        node.timestamp.to_rfc3339(),
        node.compression_depth,
        node.psi
    )
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use locus_core_rs::domain::models::{AvecState, SemanticLink, SttpNode};

    use super::graph_node_id;

    #[test]
    fn graph_node_id_is_stable() {
        let node = SttpNode {
            raw: "raw".to_string(),
            session_id: "demo".to_string(),
            tier: "raw".to_string(),
            timestamp: Utc::now(),
            compression_depth: 1,
            parent_node_id: None,
            sync_key: "sync".to_string(),
            updated_at: Utc::now(),
            source_metadata: None,
            context_summary: None,
            semantic_tags: None,
            semantic_links: Some(vec![SemanticLink {
                rel: "evaluates".to_string(),
                target: "ref:child".to_string(),
                confidence: Some(0.9),
            }]),
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedded_at: None,
            user_avec: AvecState::zero(),
            model_avec: AvecState::zero(),
            compression_avec: None,
            rho: 0.9,
            kappa: 0.9,
            psi: 2.5,
        };

        assert!(graph_node_id(&node).starts_with("n:demo|"));
    }
}
