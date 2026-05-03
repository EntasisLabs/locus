use crate::domain::memory::MemorySchemaResult;

pub struct MemorySchemaService;

impl MemorySchemaService {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self) -> MemorySchemaResult {
        MemorySchemaResult {
            schema_version: "locus-sdk.memory.v1".to_string(),
            sort_fields: vec![
                "timestamp".to_string(),
                "updated_at".to_string(),
                "psi".to_string(),
                "rho".to_string(),
                "kappa".to_string(),
            ],
            filter_fields: vec![
                "has_embedding".to_string(),
                "embedding_model".to_string(),
                "psi".to_string(),
                "rho".to_string(),
                "kappa".to_string(),
                "text_contains".to_string(),
            ],
            group_by_fields: vec![
                "session_id".to_string(),
                "tier".to_string(),
                "embedding_model".to_string(),
                "date_day".to_string(),
            ],
            fallback_policies: vec![
                "never".to_string(),
                "on_empty".to_string(),
                "always".to_string(),
            ],
            strictness_modes: vec![
                "precision".to_string(),
                "balanced".to_string(),
                "recall".to_string(),
            ],
            transform_operations: vec![
                "embed_backfill".to_string(),
                "reindex_embeddings".to_string(),
            ],
        }
    }
}

impl Default for MemorySchemaService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::MemorySchemaService;

    #[test]
    fn schema_contains_expected_core_fields() {
        let service = MemorySchemaService::new();
        let schema = service.execute();

        assert_eq!(schema.schema_version, "locus-sdk.memory.v1");
        assert!(schema.sort_fields.contains(&"timestamp".to_string()));
        assert!(schema.group_by_fields.contains(&"session_id".to_string()));
        assert!(schema.fallback_policies.contains(&"on_empty".to_string()));
        assert!(schema
            .transform_operations
            .contains(&"embed_backfill".to_string()));
    }
}
