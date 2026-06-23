use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore};
use locus_core_rs::domain::models::{NodeQuery, SttpNode};
use locus_core_rs::storage::derive_tenant_id_from_session;

use crate::application::memory_filters::{
    build_session_filter, node_matches_common_filters, resolve_indexed_sync_keys,
};
use crate::domain::memory::{
    MemoryFindRequest, MemoryFindResult, MemorySortField, SortDirection, clamp_limit,
};

pub struct MemoryFindService {
    store: Arc<dyn NodeStore>,
    semantic_index: Option<Arc<dyn SemanticIndexStore>>,
}

impl MemoryFindService {
    /// Create a deterministic memory finder over a shared node store.
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
                query_limit,
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
    use std::cmp::Ordering;

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
