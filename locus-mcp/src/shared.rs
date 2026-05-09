use chrono::{DateTime, Utc};
use locus_core_rs::EmbeddingMigrationMode;
use serde_json::{Value, json};

pub(crate) fn parse_utc_required(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|_| format!("{field} must be an ISO8601 UTC datetime"))
}

pub(crate) fn parse_utc_optional(
    value: Option<&str>,
    field: &str,
) -> Result<Option<DateTime<Utc>>, String> {
    match value {
        Some(raw) => parse_utc_required(raw, field).map(Some),
        None => Ok(None),
    }
}

pub(crate) fn validate_limit(limit: usize, field: &str) -> Result<usize, String> {
    if (1..=200).contains(&limit) {
        Ok(limit)
    } else {
        Err(format!("{field} must be between 1 and 200"))
    }
}

pub(crate) fn validate_batch_size(batch_size: usize) -> Result<usize, String> {
    if (1..=500).contains(&batch_size) {
        Ok(batch_size)
    } else {
        Err("batch_size must be between 1 and 500".to_string())
    }
}

pub(crate) fn validate_max_nodes(max_nodes: usize) -> Result<usize, String> {
    if (1..=50000).contains(&max_nodes) {
        Ok(max_nodes)
    } else {
        Err("max_nodes must be between 1 and 50000".to_string())
    }
}

pub(crate) fn parse_migration_mode(value: Option<&str>) -> Result<EmbeddingMigrationMode, String> {
    match value
        .unwrap_or("missing_only")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "missing_only" => Ok(EmbeddingMigrationMode::MissingOnly),
        "reindex_all" => Ok(EmbeddingMigrationMode::ReindexAll),
        _ => Err("mode must be one of: missing_only, reindex_all".to_string()),
    }
}

pub(crate) fn mode_to_string(mode: EmbeddingMigrationMode) -> &'static str {
    match mode {
        EmbeddingMigrationMode::MissingOnly => "missing_only",
        EmbeddingMigrationMode::ReindexAll => "reindex_all",
    }
}

pub(crate) fn expanded_limit(limit: usize) -> usize {
    limit.saturating_mul(5).clamp(1, 200)
}

pub(crate) fn normalize_tiers(tiers: &[String]) -> Vec<String> {
    tiers
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
}

pub(crate) fn normalize_context_keywords(keywords: Option<&[String]>) -> Vec<String> {
    keywords
        .unwrap_or(&[])
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
}

fn context_keyword_score(node: &locus_core_rs::SttpNode, keywords: &[String]) -> usize {
    let summary = node
        .context_summary
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let session_id = node.session_id.to_ascii_lowercase();

    keywords
        .iter()
        .filter(|keyword| {
            let needle = keyword.as_str();
            summary.contains(needle) || session_id.contains(needle)
        })
        .count()
}

pub(crate) fn filter_nodes_by_context_keywords(
    nodes: &[locus_core_rs::SttpNode],
    keywords: &[String],
    limit: usize,
) -> Vec<locus_core_rs::SttpNode> {
    let mut scored = nodes
        .iter()
        .filter_map(|node| {
            let score = context_keyword_score(node, keywords);
            if score == 0 {
                None
            } else {
                Some((score, node.timestamp, node.clone()))
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, node)| node)
        .collect::<Vec<_>>()
}

pub(crate) fn to_json_string(value: Value) -> String {
    match serde_json::to_string(&value) {
        Ok(serialized) => serialized,
        Err(err) => tool_error("SerializationFailure", &err.to_string()),
    }
}

pub(crate) fn tool_error(code: &str, message: &str) -> String {
    to_json_string(json!({
        "error": {
            "code": code,
            "message": message,
            "model_guidance": schema_first_guidance_payload(
                "If this error happened during payload construction, call get_schema first and align request shape."
            ),
        }
    }))
}

pub(crate) fn infer_store_error_code(message: &str) -> &'static str {
    let normalized = message.trim().to_ascii_lowercase();

    if normalized.starts_with("parsefailure") {
        "StrictTypedIrParseFailure"
    } else if normalized.starts_with("ratelimited") {
        "StoreRateLimited"
    } else if normalized.starts_with("storefailure") {
        "StoreFailure"
    } else if normalized.contains("strict profile") {
        "StrictTypedIrPolicyViolation"
    } else {
        "StoreContextFailure"
    }
}

pub(crate) fn strict_typed_ir_profile_name() -> &'static str {
    "strict_typed_ir"
}

fn schema_tool_name() -> &'static str {
    "get_schema"
}

pub(crate) fn schema_first_guidance_payload(summary: &str) -> Value {
    json!({
        "summary": summary,
        "recommended_first_tool": schema_tool_name(),
        "recommended_next_steps": [
            "call get_schema",
            "verify payload layers provenance->envelope->content->metrics",
            "ensure required typed-ir keys/enums/numerics are present before retry"
        ],
        "ingest_profile_policy": strict_typed_ir_profile_name(),
    })
}

pub(crate) fn avec_to_json(avec: locus_core_rs::AvecState) -> Value {
    json!({
        "stability": avec.stability,
        "friction": avec.friction,
        "logic": avec.logic,
        "autonomy": avec.autonomy,
        "psi": avec.psi(),
    })
}

pub(crate) fn sttp_node_to_json(node: &locus_core_rs::SttpNode) -> Value {
    json!({
        "raw": node.raw,
        "session_id": node.session_id,
        "tier": node.tier,
        "timestamp": node.timestamp.to_rfc3339(),
        "compression_depth": node.compression_depth,
        "parent_node_id": node.parent_node_id,
        "sync_key": node.sync_key,
        "updated_at": node.updated_at.to_rfc3339(),
        "source_metadata": node.source_metadata,
        "context_summary": node.context_summary,
        "has_embedding": node.embedding.as_ref().map(|values| !values.is_empty()).unwrap_or(false),
        "embedding_model": node.embedding_model,
        "embedding_dimensions": node.embedding_dimensions,
        "embedded_at": node.embedded_at.map(|value| value.to_rfc3339()),
        "user_avec": avec_to_json(node.user_avec),
        "model_avec": avec_to_json(node.model_avec),
        "compression_avec": node.compression_avec.map(avec_to_json),
        "rho": node.rho,
        "kappa": node.kappa,
        "psi": node.psi,
    })
}
