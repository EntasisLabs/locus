use std::cmp::Ordering;
use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::domain::contracts::NodeStore;
use locus_core_rs::domain::models::{NodeQuery, SttpNode};

use crate::application::memory_filters::{build_session_filter, node_matches_common_filters};
use crate::domain::memory::{
    MemoryFindRequest, MemoryFindResult, MemorySortField, SortDirection, clamp_limit,
};

pub struct MemoryFindService {
    store: Arc<dyn NodeStore>,
}

impl MemoryFindService {
    /// Create a deterministic memory finder over a shared node store.
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self { store }
    }

    /// Run predicate-based retrieval with stable sorting and pagination semantics.
    ///
    /// This operation does not apply resonance scoring; it filters, sorts,
    /// and truncates nodes based only on explicit request criteria.
    pub async fn execute(&self, request: &MemoryFindRequest) -> Result<MemoryFindResult> {
        let limit = clamp_limit(request.page.limit);
        let query_limit = (limit.saturating_mul(5)).clamp(1, 5000);

        let single_session = request
            .scope
            .session_ids
            .as_deref()
            .filter(|sessions| sessions.len() == 1)
            .and_then(|sessions| sessions.first().cloned());

        let mut nodes = self
            .store
            .query_nodes_async(NodeQuery {
                limit: query_limit,
                session_id: single_session,
                from_utc: request.scope.from_utc,
                to_utc: request.scope.to_utc,
                tiers: request.scope.tiers.clone(),
            })
            .await?;

        let session_filter = build_session_filter(&request.scope);

        nodes.retain(|node| {
            node_matches_common_filters(node, &request.scope, &request.filter, session_filter.as_ref())
        });
        sort_nodes(&mut nodes, request.sort.field, request.sort.direction);

        let has_more = nodes.len() > limit;
        nodes.truncate(limit);

        let next_cursor = nodes
            .last()
            .map(|node| format!("{}|{}", node.updated_at.to_rfc3339(), node.sync_key));

        Ok(MemoryFindResult {
            retrieved: nodes.len(),
            nodes,
            has_more,
            next_cursor,
        })
    }
}

fn sort_nodes(nodes: &mut [SttpNode], field: MemorySortField, direction: SortDirection) {
    nodes.sort_by(|left, right| {
        let ord = match field {
            MemorySortField::Timestamp => left.timestamp.cmp(&right.timestamp),
            MemorySortField::UpdatedAt => left.updated_at.cmp(&right.updated_at),
            MemorySortField::Psi => left.psi.partial_cmp(&right.psi).unwrap_or(Ordering::Equal),
            MemorySortField::Rho => left.rho.partial_cmp(&right.rho).unwrap_or(Ordering::Equal),
            MemorySortField::Kappa => left
                .kappa
                .partial_cmp(&right.kappa)
                .unwrap_or(Ordering::Equal),
        };

        match direction {
            SortDirection::Asc => ord,
            SortDirection::Desc => ord.reverse(),
        }
    });
}
