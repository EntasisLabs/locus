use crate::domain::models::{ConnectorMetadata, SemanticLink};
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Error as DeError};
use serde_json::Value;

fn deserialize_optional_string_vec<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_vec(deserializer)
}

fn deserialize_optional_semantic_links<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<SemanticLink>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_vec(deserializer)
}

fn deserialize_optional_vec<'de, T, D>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) if raw.trim().eq_ignore_ascii_case("null") => Ok(None),
        Some(Value::Array(items)) if items.is_empty() => Ok(None),
        Some(other) => {
            let items: Vec<T> = serde_json::from_value(other).map_err(DeError::custom)?;
            if items.is_empty() {
                Ok(None)
            } else {
                Ok(Some(items))
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealNodeRecord {
    #[serde(rename = "SessionId")]
    pub session_id: String,
    #[serde(rename = "Raw")]
    pub raw: String,
    #[serde(rename = "Tier")]
    pub tier: String,
    #[serde(rename = "Timestamp")]
    pub timestamp: String,
    #[serde(rename = "CompressionDepth")]
    pub compression_depth: i32,
    #[serde(rename = "ParentNodeId", default)]
    pub parent_node_id: Option<String>,
    #[serde(rename = "SyncKey", default)]
    pub sync_key: Option<String>,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: Option<String>,
    #[serde(rename = "SourceMetadata", default)]
    pub source_metadata: Option<ConnectorMetadata>,
    #[serde(rename = "ContextSummary", default)]
    pub context_summary: Option<String>,
    #[serde(rename = "SemanticTags", default, deserialize_with = "deserialize_optional_string_vec")]
    pub semantic_tags: Option<Vec<String>>,
    #[serde(rename = "SemanticLinks", default, deserialize_with = "deserialize_optional_semantic_links")]
    pub semantic_links: Option<Vec<SemanticLink>>,
    #[serde(rename = "Embedding", default)]
    pub embedding: Option<Vec<f32>>,
    #[serde(rename = "EmbeddingModel", default)]
    pub embedding_model: Option<String>,
    #[serde(rename = "EmbeddingDimensions", default)]
    pub embedding_dimensions: Option<usize>,
    #[serde(rename = "EmbeddedAt", default)]
    pub embedded_at: Option<String>,
    #[serde(rename = "Psi", default)]
    pub psi: f64,
    #[serde(rename = "Rho", default)]
    pub rho: f64,
    #[serde(rename = "Kappa", default)]
    pub kappa: f64,
    #[serde(rename = "UserStability", default)]
    pub user_stability: f64,
    #[serde(rename = "UserFriction", default)]
    pub user_friction: f64,
    #[serde(rename = "UserLogic", default)]
    pub user_logic: f64,
    #[serde(rename = "UserAutonomy", default)]
    pub user_autonomy: f64,
    #[serde(rename = "UserPsi", default)]
    pub user_psi: f64,
    #[serde(rename = "ModelStability", default)]
    pub model_stability: f64,
    #[serde(rename = "ModelFriction", default)]
    pub model_friction: f64,
    #[serde(rename = "ModelLogic", default)]
    pub model_logic: f64,
    #[serde(rename = "ModelAutonomy", default)]
    pub model_autonomy: f64,
    #[serde(rename = "ModelPsi", default)]
    pub model_psi: f64,
    #[serde(rename = "CompStability", default)]
    pub comp_stability: f64,
    #[serde(rename = "CompFriction", default)]
    pub comp_friction: f64,
    #[serde(rename = "CompLogic", default)]
    pub comp_logic: f64,
    #[serde(rename = "CompAutonomy", default)]
    pub comp_autonomy: f64,
    #[serde(rename = "CompPsi", default)]
    pub comp_psi: f64,
    #[serde(rename = "ResonanceDelta", default)]
    pub resonance_delta: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealSemanticTagRecord {
    #[serde(rename = "TenantId", default)]
    pub tenant_id: String,
    #[serde(rename = "SessionId", default)]
    pub session_id: String,
    #[serde(rename = "NodeId", default)]
    pub node_id: String,
    #[serde(rename = "SyncKey", default)]
    pub sync_key: String,
    #[serde(rename = "Tag", default)]
    pub tag: String,
    #[serde(rename = "Embedding", default)]
    pub embedding: Option<Vec<f32>>,
    #[serde(rename = "EmbeddingModel", default)]
    pub embedding_model: Option<String>,
    #[serde(rename = "EmbeddingDimensions", default)]
    pub embedding_dimensions: Option<usize>,
    #[serde(rename = "EmbeddedAt", default)]
    pub embedded_at: Option<String>,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealTagVocabularyRecord {
    #[serde(rename = "Tag", default)]
    pub tag: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealSyncKeyRecord {
    #[serde(rename = "SyncKey", default)]
    pub sync_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealAvecRecord {
    #[serde(default)]
    pub stability: f32,
    #[serde(default)]
    pub friction: f32,
    #[serde(default)]
    pub logic: f32,
    #[serde(default)]
    pub autonomy: f32,
    #[serde(default)]
    pub psi: f32,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealTriggerRecord {
    pub trigger: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealExistingNodeRecord {
    #[serde(rename = "Id", default)]
    pub id: Value,
    #[serde(rename = "SourceMetadata", default)]
    pub source_metadata: Option<ConnectorMetadata>,
    #[serde(rename = "ContextSummary", default)]
    pub context_summary: Option<String>,
    #[serde(rename = "Embedding", default)]
    pub embedding: Option<Vec<f32>>,
    #[serde(rename = "EmbeddingModel", default)]
    pub embedding_model: Option<String>,
    #[serde(rename = "EmbeddingDimensions", default)]
    pub embedding_dimensions: Option<usize>,
    #[serde(rename = "EmbeddedAt", default)]
    pub embedded_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurrealCheckpointRecord {
    #[serde(rename = "SessionId")]
    pub session_id: String,
    #[serde(rename = "ConnectorId")]
    pub connector_id: String,
    #[serde(rename = "CursorUpdatedAt", default)]
    pub cursor_updated_at: Option<String>,
    #[serde(rename = "CursorSyncKey", default)]
    pub cursor_sync_key: Option<String>,
    #[serde(rename = "UpdatedAt")]
    pub updated_at: String,
    #[serde(rename = "Metadata", default)]
    pub metadata: Option<ConnectorMetadata>,
}

#[cfg(test)]
mod tests {
    use super::SurrealNodeRecord;
    use serde_json::json;

    #[test]
    fn surreal_node_record_treats_null_semantic_fields_as_absent() {
        let row = json!({
            "SessionId": "s1",
            "Raw": "raw",
            "Tier": "raw",
            "Timestamp": "2026-03-05T06:30:00Z",
            "CompressionDepth": 1,
            "SemanticTags": null,
            "SemanticLinks": null,
        });

        let record: SurrealNodeRecord = serde_json::from_value(row).expect("should deserialize");
        assert_eq!(record.semantic_tags, None);
        assert_eq!(record.semantic_links, None);
    }

    #[test]
    fn surreal_node_record_treats_string_null_semantic_fields_as_absent() {
        let row = json!({
            "SessionId": "s1",
            "Raw": "raw",
            "Tier": "raw",
            "Timestamp": "2026-03-05T06:30:00Z",
            "CompressionDepth": 1,
            "SemanticTags": "null",
            "SemanticLinks": "null",
        });

        let record: SurrealNodeRecord = serde_json::from_value(row).expect("should deserialize");
        assert_eq!(record.semantic_tags, None);
        assert_eq!(record.semantic_links, None);
    }
}
