use locus_core_rs::{EmbeddingMigrationFilter, EmbeddingMigrationPreviewRequest};
use serde_json::json;
use tracing::error;

use crate::{
    PreviewEmbeddingMigrationRequest, SttpMcpServer, normalize_tiers, parse_utc_optional,
    to_json_string, tool_error, validate_limit, validate_max_nodes,
};

pub(crate) async fn execute(
    server: &SttpMcpServer,
    request: PreviewEmbeddingMigrationRequest,
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
    let sample_limit = match validate_limit(request.sample_limit, "sample_limit") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };
    let max_nodes = match validate_max_nodes(request.max_nodes) {
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
        .preview_async(EmbeddingMigrationPreviewRequest {
            filter,
            sample_limit,
            max_nodes,
        })
        .await
    {
        Ok(result) => to_json_string(json!({
            "total_candidates": result.total_candidates,
            "provider_available": result.provider_available,
            "provider_model": result.provider_model,
            "sample": result
                .sample
                .iter()
                .map(|sample| json!({
                    "sync_key": sample.sync_key,
                    "session_id": sample.session_id,
                    "tier": sample.tier,
                    "has_embedding": sample.has_embedding,
                    "embedding_model": sample.embedding_model,
                    "embedding_dimensions": sample.embedding_dimensions,
                    "embedded_at": sample.embedded_at.map(|value| value.to_rfc3339()),
                    "updated_at": sample.updated_at.to_rfc3339(),
                    "context_summary": sample.context_summary,
                }))
                .collect::<Vec<_>>(),
        })),
        Err(err) => {
            error!(error = %err, "preview_embedding_migration failed");
            tool_error("MigrationPreviewFailure", &err.to_string())
        }
    }
}
