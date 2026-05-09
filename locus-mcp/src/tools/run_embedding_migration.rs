use locus_core_rs::{EmbeddingMigrationFilter, EmbeddingMigrationRunRequest};
use serde_json::json;
use tracing::error;

use crate::{
    RunEmbeddingMigrationRequest, SttpMcpServer, mode_to_string, normalize_tiers,
    parse_migration_mode, parse_utc_optional, to_json_string, tool_error, validate_batch_size,
    validate_max_nodes,
};

pub(crate) async fn execute(
    server: &SttpMcpServer,
    request: RunEmbeddingMigrationRequest,
) -> String {
    let from_utc = match parse_utc_optional(request.from_utc.as_deref(), "from_utc") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidDate", &message),
    };
    let to_utc = match parse_utc_optional(request.to_utc.as_deref(), "to_utc") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidDate", &message),
    };
    let tiers = request
        .tiers
        .as_ref()
        .map(|values| normalize_tiers(values.as_slice()));
    let batch_size = match validate_batch_size(request.batch_size) {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };
    let max_nodes = match validate_max_nodes(request.max_nodes) {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };
    let mode = match parse_migration_mode(request.mode.as_deref()) {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };

    let filter = EmbeddingMigrationFilter {
        session_id: request.session_id,
        from_utc,
        to_utc,
        tiers,
        has_embedding: request.has_embedding,
        embedding_model: request.embedding_model,
        sync_keys: request.sync_keys,
    };

    match server
        .embedding_migration
        .run_async(EmbeddingMigrationRunRequest {
            filter,
            mode,
            dry_run: request.dry_run,
            batch_size,
            max_nodes,
        })
        .await
    {
        Ok(result) => to_json_string(json!({
            "scanned": result.scanned,
            "selected": result.selected,
            "updated": result.updated,
            "skipped": result.skipped,
            "failed": result.failed,
            "duplicate": result.duplicate,
            "started_at": result.started_at.to_rfc3339(),
            "completed_at": result.completed_at.to_rfc3339(),
            "provider_model": result.provider_model,
            "dry_run": request.dry_run,
            "mode": mode_to_string(mode),
            "failure_reasons": result.failure_reasons,
        })),
        Err(err) => {
            error!(error = %err, "run_embedding_migration failed");
            tool_error("MigrationRunFailure", &err.to_string())
        }
    }
}
