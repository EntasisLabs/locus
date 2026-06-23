use locus_sdk::application::memory_graph::MemoryGraphService;
use locus_sdk::domain::graph::MemoryGraphRequest;
use locus_sdk::domain::memory::{MemoryFilter, MemoryScope};
use serde_json::json;
use tracing::error;

use crate::{GetGraphRequest, SttpMcpServer, to_json_string, tool_error, validate_limit};

pub(crate) async fn execute(server: &SttpMcpServer, request: GetGraphRequest) -> String {
    let limit = match validate_limit(request.limit, "limit") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };

    let graph_service = MemoryGraphService::new(server.node_store.clone())
        .with_semantic_index(server.semantic_index.clone());

    let graph_result = match graph_service
        .execute(&MemoryGraphRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: request.session_id.map(|session| vec![session]),
                tiers: None,
                from_utc: None,
                to_utc: None,
            },
            filter: MemoryFilter {
                indexed_tags: request.semantic_tags,
                link_rel: request.link_rel,
                link_target: request.link_target,
                links_to_ref: request.links_to_ref,
                tag_prefix: request.tag_prefix,
                has_semantic_links: request.has_semantic_links,
                ..Default::default()
            },
            include_lineage: request.include_lineage.unwrap_or(true),
            include_semantic: request.include_semantic.unwrap_or(true),
            include_session_topology: request.include_session_topology.unwrap_or(true),
            rel: request.rel,
            target_prefix: request.target_prefix,
            limit,
        })
        .await
    {
        Ok(result) => result,
        Err(err) => {
            error!(error = %err, "get_graph failed");
            return tool_error("GetGraphFailure", &err.to_string());
        }
    };

    to_json_string(json!({
        "retrieved": graph_result.retrieved,
        "sessions": graph_result.sessions,
        "nodes": graph_result.nodes,
        "edges": graph_result.edges,
    }))
}
