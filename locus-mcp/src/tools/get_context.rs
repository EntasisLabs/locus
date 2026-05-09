use locus_sdk::application::memory_recall::MemoryRecallService;
use locus_sdk::domain::memory::{MemoryPage, MemoryRecallRequest, MemoryScope, MemoryScoring};
use serde_json::json;
use tracing::error;

use crate::{
    GetContextRequest, SttpMcpServer, normalize_context_keywords, normalize_tiers,
    parse_utc_optional, sttp_node_to_json, to_json_string, tool_error, validate_limit,
};

pub(crate) async fn execute(server: &SttpMcpServer, request: GetContextRequest) -> String {
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

    let limit = match validate_limit(request.limit, "limit") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidArgument", &message),
    };
    let context_keywords = normalize_context_keywords(request.context_keywords.as_deref());

    if let Some(alpha) = request.alpha {
        if !(0.0..=1.0).contains(&alpha) {
            return tool_error("InvalidArgument", "alpha must be between 0.0 and 1.0");
        }
    }
    if let Some(beta) = request.beta {
        if !(0.0..=1.0).contains(&beta) {
            return tool_error("InvalidArgument", "beta must be between 0.0 and 1.0");
        }
    }

    let alpha = request.alpha.unwrap_or(0.7);
    let beta = request.beta.unwrap_or(0.3);
    let query_text = if context_keywords.is_empty() {
        None
    } else {
        Some(context_keywords.join(" "))
    };
    let query_embedding = if context_keywords.is_empty() {
        None
    } else {
        server.embed_context_keywords(&context_keywords).await
    };

    let recall_service = MemoryRecallService::new(server.node_store.clone());
    let recall_result = match recall_service
        .execute(&MemoryRecallRequest {
            scope: MemoryScope {
                tenant_id: None,
                session_ids: request.session_id.map(|session| vec![session]),
                tiers,
                from_utc,
                to_utc,
            },
            page: MemoryPage {
                limit,
                cursor: None,
            },
            scoring: MemoryScoring {
                alpha,
                beta,
                ..Default::default()
            },
            current_avec: Some(locus_core_rs::AvecState {
                stability: request.stability,
                friction: request.friction,
                logic: request.logic,
                autonomy: request.autonomy,
            }),
            query_text,
            query_embedding,
            ..Default::default()
        })
        .await
    {
        Ok(result) => result,
        Err(err) => {
            error!(error = %err, "get_context failed");
            return tool_error("GetContextFailure", &err.to_string());
        }
    };

    to_json_string(json!({
        "retrieved": recall_result.retrieved,
        "psi_range": {
            "min": recall_result.psi_range.min,
            "max": recall_result.psi_range.max,
            "average": recall_result.psi_range.average,
        },
        "nodes": recall_result
            .nodes
            .iter()
            .map(sttp_node_to_json)
            .collect::<Vec<_>>(),
    }))
}
