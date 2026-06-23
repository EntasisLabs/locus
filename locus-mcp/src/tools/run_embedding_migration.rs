use locus_core_rs::{EmbeddingMigrationFilter, EmbeddingMigrationRunRequest};
use locus_sdk::application::memory_transform::MemoryTransformService;
use locus_sdk::domain::memory::{
    MemoryFilter, MemoryScope, MemoryTransformOperation, MemoryTransformRequest,
};
use locus_sdk::infrastructure::registry::InMemoryAiProviderRegistry;
use locus_sdk::infrastructure::sttp_native::embedding_provider_adapter::SttpEmbeddingProviderAdapter;
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

    let mode_raw = request
        .mode
        .as_deref()
        .unwrap_or("missing_only")
        .trim()
        .to_ascii_lowercase();

    if matches!(mode_raw.as_str(), "tags" | "tag" | "embed_tag_backfill" | "both") {
        return run_tag_transform(server, request, from_utc, to_utc, tiers, batch_size, max_nodes)
            .await;
    }

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

async fn run_tag_transform(
    server: &SttpMcpServer,
    request: RunEmbeddingMigrationRequest,
    from_utc: Option<chrono::DateTime<chrono::Utc>>,
    to_utc: Option<chrono::DateTime<chrono::Utc>>,
    tiers: Option<Vec<String>>,
    batch_size: usize,
    max_nodes: usize,
) -> String {
    let mode_raw = request
        .mode
        .as_deref()
        .unwrap_or("tags")
        .trim()
        .to_ascii_lowercase();
    let operation = if mode_raw == "both" {
        MemoryTransformOperation::ReindexTagEmbeddings
    } else {
        MemoryTransformOperation::EmbedTagBackfill
    };

    let mut registry = InMemoryAiProviderRegistry::new();
    if let Some(provider) = server.embedding_provider.as_ref() {
        registry.register(SttpEmbeddingProviderAdapter::new(
            "mcp-embedding",
            provider.clone(),
        ));
    }

    let transform_service = MemoryTransformService::new(
        server.node_store.clone(),
        std::sync::Arc::new(registry),
    )
    .with_semantic_index(server.semantic_index.clone());

    match transform_service
        .execute(&MemoryTransformRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: request.session_id.map(|session| vec![session]),
                tiers,
                from_utc,
                to_utc,
            },
            filter: MemoryFilter::default(),
            operation,
            dry_run: request.dry_run,
            batch_size,
            max_nodes,
            provider_id: server
                .embedding_provider
                .as_ref()
                .map(|_| "mcp-embedding".to_string()),
            model: server
                .embedding_provider
                .as_ref()
                .map(|provider| provider.model_name().to_string()),
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
            "provider_model": server.embedding_provider.as_ref().map(|provider| provider.model_name().to_string()),
            "dry_run": request.dry_run,
            "mode": mode_raw,
            "failure_reasons": result.failures,
        })),
        Err(err) => {
            error!(error = %err, "run_embedding_migration tag transform failed");
            tool_error("MigrationRunFailure", &err.to_string())
        }
    }
}
