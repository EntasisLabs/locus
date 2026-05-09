use locus_sdk::application::memory_find::MemoryFindService;
use locus_sdk::domain::memory::{MemoryFindRequest, MemoryPage, MemoryScope};
use serde_json::json;
use tracing::error;

use crate::{
    ListNodesRequest, SttpMcpServer, expanded_limit, filter_nodes_by_context_keywords,
    normalize_context_keywords, sttp_node_to_json, to_json_string, tool_error, validate_limit,
};

pub(crate) async fn execute(server: &SttpMcpServer, request: ListNodesRequest) -> String {
    let limit = match validate_limit(request.limit, "limit") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };
    let context_keywords = normalize_context_keywords(request.context_keywords.as_deref());
    let query_limit = if context_keywords.is_empty() {
        limit
    } else {
        expanded_limit(limit)
    };

    let find_service = MemoryFindService::new(server.node_store.clone());
    let find_result = match find_service
        .execute(&MemoryFindRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: request.session_id.map(|session| vec![session]),
                tiers: None,
                from_utc: None,
                to_utc: None,
            },
            page: MemoryPage {
                limit: query_limit,
                cursor: None,
            },
            ..Default::default()
        })
        .await
    {
        Ok(result) => result,
        Err(err) => {
            error!(error = %err, "list_nodes failed");
            return tool_error("ListNodesFailure", &err.to_string());
        }
    };

    let nodes = if context_keywords.is_empty() {
        find_result
            .nodes
            .into_iter()
            .take(limit)
            .collect::<Vec<_>>()
    } else {
        filter_nodes_by_context_keywords(&find_result.nodes, &context_keywords, limit)
    };

    to_json_string(json!({
        "retrieved": nodes.len(),
        "nodes": nodes
            .iter()
            .map(sttp_node_to_json)
            .collect::<Vec<_>>()
    }))
}
