use locus_sdk::application::memory_evict::MemoryEvictService;
use locus_sdk::domain::evict::{MemoryEvictMode, MemoryEvictRequest};
use locus_sdk::domain::memory::{MemoryFilter, MemoryScope};
use serde_json::json;
use tracing::error;

use crate::{EvictNodesRequest, SttpMcpServer, to_json_string, tool_error};

pub(crate) async fn execute(server: &SttpMcpServer, request: EvictNodesRequest) -> String {
    let session_id = request.session_id.trim();
    if session_id.is_empty() {
        return tool_error("InvalidArgument", "session_id is required");
    }

    let mode = if request.purge_session.unwrap_or(false) {
        MemoryEvictMode::PurgeSession
    } else if request
        .sync_keys
        .as_ref()
        .is_some_and(|keys| !keys.is_empty())
    {
        MemoryEvictMode::BySyncKeys
    } else if request
        .node_ids
        .as_ref()
        .is_some_and(|ids| !ids.is_empty())
    {
        MemoryEvictMode::ByNodeIds
    } else if request.semantic_tags.is_some()
        || request.link_rel.is_some()
        || request.link_target.is_some()
        || request.links_to_ref.is_some()
        || request.tag_prefix.is_some()
        || request.has_semantic_links.is_some()
    {
        MemoryEvictMode::ByFilter
    } else {
        return tool_error(
            "InvalidArgument",
            "provide sync_keys, node_ids, semantic filter fields, or purge_session=true",
        );
    };

    let purge_session = matches!(mode, MemoryEvictMode::PurgeSession);
    let include_calibration = request.include_calibration.unwrap_or(purge_session);
    let include_checkpoints = request.include_checkpoints.unwrap_or(purge_session);

    let evict_service = MemoryEvictService::new(server.node_store.clone())
        .with_semantic_index(server.semantic_index.clone());

    let result = match evict_service
        .execute(&MemoryEvictRequest {
            mode,
            scope: MemoryScope {
                session_ids: Some(vec![session_id.to_string()]),
                ..Default::default()
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
            sync_keys: request.sync_keys,
            node_ids: request.node_ids,
            dry_run: request.dry_run.unwrap_or(false),
            force: request.force.unwrap_or(false),
            max_nodes: request.max_nodes.unwrap_or(5000),
            include_calibration,
            include_checkpoints,
        })
        .await
    {
        Ok(result) => result,
        Err(err) => {
            error!(error = %err, "evict_nodes failed");
            return tool_error("EvictNodesFailure", &err.to_string());
        }
    };

    to_json_string(json!({
        "dryRun": result.dry_run,
        "deleted": result.deleted,
        "blocked": result.blocked,
        "notFound": result.not_found,
        "skipped": result.skipped,
        "wouldDelete": result.would_delete,
        "calibrationsDeleted": result.calibrations_deleted,
        "checkpointsDeleted": result.checkpoints_deleted,
        "records": result.records,
    }))
}
