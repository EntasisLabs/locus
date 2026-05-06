pub const INIT_SCHEMA_QUERY: &str = r#"
            DEFINE TABLE IF NOT EXISTS temporal_node SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS tenant_id         ON temporal_node TYPE string;
            DEFINE FIELD IF NOT EXISTS session_id        ON temporal_node TYPE string;
            DEFINE FIELD IF NOT EXISTS raw               ON temporal_node TYPE string;
            DEFINE FIELD IF NOT EXISTS tier              ON temporal_node TYPE string;
            DEFINE FIELD IF NOT EXISTS timestamp         ON temporal_node TYPE datetime;
            DEFINE FIELD IF NOT EXISTS compression_depth ON temporal_node TYPE int;
            DEFINE FIELD IF NOT EXISTS parent_node_id    ON temporal_node TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS sync_key          ON temporal_node TYPE string;
            DEFINE FIELD IF NOT EXISTS updated_at        ON temporal_node TYPE datetime;
            DEFINE FIELD IF NOT EXISTS source_metadata   ON temporal_node TYPE option<object>;
            DEFINE FIELD IF NOT EXISTS context_summary   ON temporal_node TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS embedding         ON temporal_node TYPE option<array<float>>;
            DEFINE FIELD IF NOT EXISTS embedding_model   ON temporal_node TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS embedding_dimensions ON temporal_node TYPE option<int>;
            DEFINE FIELD IF NOT EXISTS embedded_at       ON temporal_node TYPE option<datetime>;
            DEFINE FIELD IF NOT EXISTS psi               ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS rho               ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS kappa             ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS user_stability    ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS user_friction     ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS user_logic        ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS user_autonomy     ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS user_psi          ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS model_stability   ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS model_friction    ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS model_logic       ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS model_autonomy    ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS model_psi         ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS comp_stability    ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS comp_friction     ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS comp_logic        ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS comp_autonomy     ON temporal_node TYPE float;
            DEFINE FIELD IF NOT EXISTS comp_psi          ON temporal_node TYPE float;

            DEFINE TABLE IF NOT EXISTS calibration SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS tenant_id   ON calibration TYPE string;
            DEFINE FIELD IF NOT EXISTS session_id  ON calibration TYPE string;
            DEFINE FIELD IF NOT EXISTS stability   ON calibration TYPE float;
            DEFINE FIELD IF NOT EXISTS friction    ON calibration TYPE float;
            DEFINE FIELD IF NOT EXISTS logic       ON calibration TYPE float;
            DEFINE FIELD IF NOT EXISTS autonomy    ON calibration TYPE float;
            DEFINE FIELD IF NOT EXISTS psi         ON calibration TYPE float;
            DEFINE FIELD IF NOT EXISTS trigger     ON calibration TYPE string;
            DEFINE FIELD IF NOT EXISTS created_at  ON calibration TYPE datetime;

            DEFINE TABLE IF NOT EXISTS sync_checkpoint SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS tenant_id          ON sync_checkpoint TYPE string;
            DEFINE FIELD IF NOT EXISTS session_id         ON sync_checkpoint TYPE string;
            DEFINE FIELD IF NOT EXISTS connector_id       ON sync_checkpoint TYPE string;
            DEFINE FIELD IF NOT EXISTS cursor_updated_at  ON sync_checkpoint TYPE option<datetime>;
            DEFINE FIELD IF NOT EXISTS cursor_sync_key    ON sync_checkpoint TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS metadata           ON sync_checkpoint TYPE option<object>;
            DEFINE FIELD IF NOT EXISTS updated_at         ON sync_checkpoint TYPE datetime;

            DEFINE INDEX IF NOT EXISTS idx_node_session ON temporal_node FIELDS session_id;
            DEFINE INDEX IF NOT EXISTS idx_node_tenant_session ON temporal_node FIELDS tenant_id, session_id;
            DEFINE INDEX IF NOT EXISTS idx_node_tier ON temporal_node FIELDS tier;
            DEFINE INDEX IF NOT EXISTS idx_node_timestamp ON temporal_node FIELDS timestamp;
            DEFINE INDEX IF NOT EXISTS idx_node_change_cursor ON temporal_node FIELDS tenant_id, session_id, updated_at, sync_key;
            DEFINE INDEX IF NOT EXISTS idx_node_sync_identity ON temporal_node FIELDS tenant_id, session_id, sync_key UNIQUE;
            DEFINE INDEX IF NOT EXISTS idx_cal_session ON calibration FIELDS session_id;
            DEFINE INDEX IF NOT EXISTS idx_cal_tenant_session ON calibration FIELDS tenant_id, session_id;
            DEFINE INDEX IF NOT EXISTS idx_checkpoint_scope ON sync_checkpoint FIELDS tenant_id, session_id, connector_id UNIQUE;
            SELECT * FROM calibration LIMIT 0;
            "#;

pub fn query_nodes_query(where_clause: &str, capped_limit: usize) -> String {
    format!(
        r#"
            SELECT
                tenant_id AS TenantId,
                session_id AS SessionId,
                raw AS Raw,
                tier AS Tier,
                timestamp AS Timestamp,
                compression_depth AS CompressionDepth,
                parent_node_id AS ParentNodeId,
                sync_key AS SyncKey,
                updated_at AS UpdatedAt,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt,
                psi AS Psi,
                rho AS Rho,
                kappa AS Kappa,
                user_stability AS UserStability,
                user_friction AS UserFriction,
                user_logic AS UserLogic,
                user_autonomy AS UserAutonomy,
                user_psi AS UserPsi,
                model_stability AS ModelStability,
                model_friction AS ModelFriction,
                model_logic AS ModelLogic,
                model_autonomy AS ModelAutonomy,
                model_psi AS ModelPsi,
                comp_stability AS CompStability,
                comp_friction AS CompFriction,
                comp_logic AS CompLogic,
                comp_autonomy AS CompAutonomy,
                comp_psi AS CompPsi,
                0 AS ResonanceDelta
            FROM temporal_node
            {where_clause}
            ORDER BY Timestamp DESC
            LIMIT {capped_limit};
            "#
    )
}

pub fn create_temporal_node_query(
    record_id: &str,
    include_parent_assignment: bool,
    include_source_metadata_assignment: bool,
    include_embedding_assignment: bool,
    include_context_summary_assignment: bool,
    include_embedding_vector_assignment: bool,
    include_embedding_model_assignment: bool,
    include_embedding_dimensions_assignment: bool,
    include_embedded_at_assignment: bool,
) -> String {
    let parent_assignment = if include_parent_assignment {
        "\n                parent_node_id = $parent_node_id,"
    } else {
        ""
    };

    let source_metadata_assignment = if include_source_metadata_assignment {
        "\n                source_metadata = $source_metadata,"
    } else {
        ""
    };

    let context_summary_assignment = if include_embedding_assignment {
        let context_summary_value = if include_context_summary_assignment {
            "$context_summary"
        } else {
            "NONE"
        };
        let embedding_value = if include_embedding_vector_assignment {
            "$embedding"
        } else {
            "NONE"
        };
        let embedding_model_value = if include_embedding_model_assignment {
            "$embedding_model"
        } else {
            "NONE"
        };
        let embedding_dimensions_value = if include_embedding_dimensions_assignment {
            "$embedding_dimensions"
        } else {
            "NONE"
        };
        let embedded_at_assignment = if include_embedded_at_assignment {
            "<datetime>$embedded_at"
        } else {
            "NONE"
        };

        format!(
            "\n                context_summary = {context_summary_value},\n                embedding = {embedding_value},\n                embedding_model = {embedding_model_value},\n                embedding_dimensions = {embedding_dimensions_value},\n                embedded_at = {embedded_at_assignment},"
        )
    } else {
        "\n                context_summary = NONE,\n                embedding = NONE,\n                embedding_model = NONE,\n                embedding_dimensions = NONE,\n                embedded_at = NONE,".to_string()
    };

    format!(
        r#"
            CREATE temporal_node:`{record_id}` SET
                tenant_id = $tenant_id,
                session_id = $session_id,
                raw = $raw,
                tier = $tier,
                timestamp = <datetime>$timestamp,
                compression_depth = $compression_depth,{parent_assignment}
                sync_key = $sync_key,
                updated_at = <datetime>$updated_at,{source_metadata_assignment}
                {context_summary_assignment}
                psi = $psi,
                rho = $rho,
                kappa = $kappa,
                user_stability = $user_stability,
                user_friction = $user_friction,
                user_logic = $user_logic,
                user_autonomy = $user_autonomy,
                user_psi = $user_psi,
                model_stability = $model_stability,
                model_friction = $model_friction,
                model_logic = $model_logic,
                model_autonomy = $model_autonomy,
                model_psi = $model_psi,
                comp_stability = $comp_stability,
                comp_friction = $comp_friction,
                comp_logic = $comp_logic,
                comp_autonomy = $comp_autonomy,
                comp_psi = $comp_psi;
            "#
    )
}

pub fn get_by_resonance_query(
    current_stability: f32,
    current_friction: f32,
    current_logic: f32,
    current_autonomy: f32,
    additional_predicate: &str,
    limit: usize,
) -> String {
    let stability = format!("{current_stability:.4}");
    let friction = format!("{current_friction:.4}");
    let logic = format!("{current_logic:.4}");
    let autonomy = format!("{current_autonomy:.4}");
    let where_suffix = if additional_predicate.trim().is_empty() {
        String::new()
    } else {
        format!(" AND {}", additional_predicate.trim())
    };

    format!(
        r#"
            SELECT
                tenant_id AS TenantId,
                session_id AS SessionId,
                raw AS Raw,
                tier AS Tier,
                timestamp AS Timestamp,
                compression_depth AS CompressionDepth,
                parent_node_id AS ParentNodeId,
                sync_key AS SyncKey,
                updated_at AS UpdatedAt,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt,
                psi AS Psi,
                rho AS Rho,
                kappa AS Kappa,
                user_stability AS UserStability,
                user_friction AS UserFriction,
                user_logic AS UserLogic,
                user_autonomy AS UserAutonomy,
                user_psi AS UserPsi,
                model_stability AS ModelStability,
                model_friction AS ModelFriction,
                model_logic AS ModelLogic,
                model_autonomy AS ModelAutonomy,
                model_psi AS ModelPsi,
                comp_stability AS CompStability,
                comp_friction AS CompFriction,
                comp_logic AS CompLogic,
                comp_autonomy AS CompAutonomy,
                comp_psi AS CompPsi,
                (
                    math::abs(model_stability - {stability})
                    + math::abs(model_friction - {friction})
                    + math::abs(model_logic - {logic})
                    + math::abs(model_autonomy - {autonomy})
                ) / 4.0 AS ResonanceDelta
            FROM temporal_node
                        WHERE session_id = $session_id
                            AND (tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')
                            {where_suffix}
            ORDER BY ResonanceDelta ASC
            LIMIT {limit};
            "#
    )
}

pub fn get_by_resonance_global_query(
    current_stability: f32,
    current_friction: f32,
    current_logic: f32,
    current_autonomy: f32,
    additional_predicate: &str,
    limit: usize,
) -> String {
    let stability = format!("{current_stability:.4}");
    let friction = format!("{current_friction:.4}");
    let logic = format!("{current_logic:.4}");
    let autonomy = format!("{current_autonomy:.4}");
    let where_clause = if additional_predicate.trim().is_empty() {
        String::new()
    } else {
        format!("WHERE {}", additional_predicate.trim())
    };

    format!(
        r#"
            SELECT
                tenant_id AS TenantId,
                session_id AS SessionId,
                raw AS Raw,
                tier AS Tier,
                timestamp AS Timestamp,
                compression_depth AS CompressionDepth,
                parent_node_id AS ParentNodeId,
                sync_key AS SyncKey,
                updated_at AS UpdatedAt,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt,
                psi AS Psi,
                rho AS Rho,
                kappa AS Kappa,
                user_stability AS UserStability,
                user_friction AS UserFriction,
                user_logic AS UserLogic,
                user_autonomy AS UserAutonomy,
                user_psi AS UserPsi,
                model_stability AS ModelStability,
                model_friction AS ModelFriction,
                model_logic AS ModelLogic,
                model_autonomy AS ModelAutonomy,
                model_psi AS ModelPsi,
                comp_stability AS CompStability,
                comp_friction AS CompFriction,
                comp_logic AS CompLogic,
                comp_autonomy AS CompAutonomy,
                comp_psi AS CompPsi,
                (
                    math::abs(model_stability - {stability})
                    + math::abs(model_friction - {friction})
                    + math::abs(model_logic - {logic})
                    + math::abs(model_autonomy - {autonomy})
                ) / 4.0 AS ResonanceDelta
            FROM temporal_node
            {where_clause}
            ORDER BY ResonanceDelta ASC
            LIMIT {limit};
            "#
    )
}

pub const GET_LAST_AVEC_QUERY: &str = r#"
            SELECT stability, friction, logic, autonomy, psi, created_at
            FROM calibration
                        WHERE session_id = $session_id
                            AND (tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')
            ORDER BY created_at DESC
            LIMIT 1;
            "#;

pub const GET_TRIGGER_HISTORY_QUERY: &str = r#"
            SELECT trigger, created_at FROM calibration
                        WHERE session_id = $session_id
                            AND (tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')
            ORDER BY created_at ASC;
            "#;

pub const STORE_CALIBRATION_QUERY: &str = r#"
            CREATE calibration SET
                tenant_id = $tenant_id,
                session_id = $session_id,
                stability = $stability,
                friction = $friction,
                logic = $logic,
                autonomy = $autonomy,
                psi = $psi,
                trigger = $trigger,
                created_at = <datetime>$created_at;
            "#;

pub const SELECT_TEMPORAL_NODE_LEGACY_SYNC_QUERY: &str = r#"
            SELECT id, session_id, timestamp, sync_key, updated_at
            FROM temporal_node
            WHERE tenant_id = NONE OR tenant_id = '' OR sync_key = NONE OR sync_key = '' OR updated_at = NONE;
            "#;

pub fn update_temporal_node_legacy_sync_query(record_id: &str) -> String {
    format!(
        r#"
            UPDATE temporal_node:`{record_id}` SET
                tenant_id = $tenant_id,
                sync_key = $sync_key,
                updated_at = <datetime>$updated_at;
            "#
    )
}

pub const FIND_EXISTING_NODE_BY_SYNC_KEY_QUERY: &str = r#"
            SELECT
                id AS Id,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt
            FROM temporal_node
            WHERE session_id = $session_id
                AND sync_key = $sync_key
                AND (tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')
            LIMIT 1;
            "#;

pub const FIND_EXISTING_NODE_BY_SYNC_KEY_EXACT_QUERY: &str = r#"
            SELECT
                id AS Id,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt
            FROM temporal_node
            WHERE session_id = $session_id
                AND sync_key = $sync_key
                AND tenant_id = $tenant_id
            LIMIT 1;
            "#;

pub const FIND_EXISTING_NODE_BY_SYNC_KEY_ANY_TENANT_QUERY: &str = r#"
            SELECT
                id AS Id,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt
            FROM temporal_node
            WHERE session_id = $session_id
                AND sync_key = $sync_key
            LIMIT 1;
            "#;

pub fn update_temporal_node_sync_metadata_query(
    record_id: &str,
    clear_source_metadata: bool,
) -> String {
    let source_metadata_assignment = if clear_source_metadata {
        "source_metadata = NONE,"
    } else {
        "source_metadata = $source_metadata,"
    };

    format!(
        r#"
            UPDATE temporal_node:`{record_id}` SET
                {source_metadata_assignment}
                updated_at = <datetime>$updated_at;
            "#
    )
}

pub fn update_temporal_node_query(
    record_id: &str,
    include_parent_assignment: bool,
    include_source_metadata_assignment: bool,
    include_embedding_assignment: bool,
    include_context_summary_assignment: bool,
    include_embedding_vector_assignment: bool,
    include_embedding_model_assignment: bool,
    include_embedding_dimensions_assignment: bool,
    include_embedded_at_assignment: bool,
) -> String {
    let parent_assignment = if include_parent_assignment {
        "\n                parent_node_id = $parent_node_id,"
    } else {
        "\n                parent_node_id = NONE,"
    };

    let source_metadata_assignment = if include_source_metadata_assignment {
        "\n                source_metadata = $source_metadata,"
    } else {
        "\n                source_metadata = NONE,"
    };

    let context_summary_assignment = if include_embedding_assignment {
        let context_summary_value = if include_context_summary_assignment {
            "$context_summary"
        } else {
            "NONE"
        };
        let embedding_value = if include_embedding_vector_assignment {
            "$embedding"
        } else {
            "NONE"
        };
        let embedding_model_value = if include_embedding_model_assignment {
            "$embedding_model"
        } else {
            "NONE"
        };
        let embedding_dimensions_value = if include_embedding_dimensions_assignment {
            "$embedding_dimensions"
        } else {
            "NONE"
        };
        let embedded_at_assignment = if include_embedded_at_assignment {
            "<datetime>$embedded_at"
        } else {
            "NONE"
        };

        format!(
            "\n                context_summary = {context_summary_value},\n                embedding = {embedding_value},\n                embedding_model = {embedding_model_value},\n                embedding_dimensions = {embedding_dimensions_value},\n                embedded_at = {embedded_at_assignment},"
        )
    } else {
        "\n                context_summary = NONE,\n                embedding = NONE,\n                embedding_model = NONE,\n                embedding_dimensions = NONE,\n                embedded_at = NONE,"
            .to_string()
    };

    format!(
        r#"
            UPDATE temporal_node:`{record_id}` SET
                tenant_id = $tenant_id,
                session_id = $session_id,
                raw = $raw,
                tier = $tier,
                timestamp = <datetime>$timestamp,
                compression_depth = $compression_depth,{parent_assignment}
                sync_key = $sync_key,
                updated_at = <datetime>$updated_at,{source_metadata_assignment}
                {context_summary_assignment}
                psi = $psi,
                rho = $rho,
                kappa = $kappa,
                user_stability = $user_stability,
                user_friction = $user_friction,
                user_logic = $user_logic,
                user_autonomy = $user_autonomy,
                user_psi = $user_psi,
                model_stability = $model_stability,
                model_friction = $model_friction,
                model_logic = $model_logic,
                model_autonomy = $model_autonomy,
                model_psi = $model_psi,
                comp_stability = $comp_stability,
                comp_friction = $comp_friction,
                comp_logic = $comp_logic,
                comp_autonomy = $comp_autonomy,
                comp_psi = $comp_psi;
            "#
    )
}

pub fn query_changes_since_query(limit: usize) -> String {
    format!(
        r#"
            SELECT
                tenant_id AS TenantId,
                session_id AS SessionId,
                raw AS Raw,
                tier AS Tier,
                timestamp AS Timestamp,
                compression_depth AS CompressionDepth,
                parent_node_id AS ParentNodeId,
                sync_key AS SyncKey,
                updated_at AS UpdatedAt,
                source_metadata AS SourceMetadata,
                context_summary AS ContextSummary,
                embedding AS Embedding,
                embedding_model AS EmbeddingModel,
                embedding_dimensions AS EmbeddingDimensions,
                embedded_at AS EmbeddedAt,
                psi AS Psi,
                rho AS Rho,
                kappa AS Kappa,
                user_stability AS UserStability,
                user_friction AS UserFriction,
                user_logic AS UserLogic,
                user_autonomy AS UserAutonomy,
                user_psi AS UserPsi,
                model_stability AS ModelStability,
                model_friction AS ModelFriction,
                model_logic AS ModelLogic,
                model_autonomy AS ModelAutonomy,
                model_psi AS ModelPsi,
                comp_stability AS CompStability,
                comp_friction AS CompFriction,
                comp_logic AS CompLogic,
                comp_autonomy AS CompAutonomy,
                comp_psi AS CompPsi,
                0 AS ResonanceDelta
            FROM temporal_node
            WHERE session_id = $session_id
                AND (tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')
                AND (
                    NOT $include_cursor
                    OR updated_at > <datetime>$cursor_updated_at
                    OR (
                        updated_at = <datetime>$cursor_updated_at
                        AND sync_key > $cursor_sync_key
                    )
                )
            ORDER BY updated_at ASC, sync_key ASC
            LIMIT {limit};
            "#
    )
}

pub const GET_SYNC_CHECKPOINT_QUERY: &str = r#"
            SELECT
                session_id AS SessionId,
                connector_id AS ConnectorId,
                cursor_updated_at AS CursorUpdatedAt,
                cursor_sync_key AS CursorSyncKey,
                updated_at AS UpdatedAt,
                metadata AS Metadata
            FROM sync_checkpoint
            WHERE session_id = $session_id
                AND connector_id = $connector_id
                AND (tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = '')
            LIMIT 1;
            "#;

pub fn upsert_sync_checkpoint_query(record_id: &str, include_metadata_assignment: bool) -> String {
    let metadata_assignment = if include_metadata_assignment {
        "metadata = $metadata,"
    } else {
        "metadata = NONE,"
    };

    format!(
        r#"
            UPSERT sync_checkpoint:`{record_id}` SET
                tenant_id = $tenant_id,
                session_id = $session_id,
                connector_id = $connector_id,
                cursor_updated_at = <datetime>$cursor_updated_at,
                cursor_sync_key = $cursor_sync_key,
                {metadata_assignment}
                updated_at = <datetime>$updated_at;
            "#
    )
}

pub const SELECT_CALIBRATION_MISSING_TENANT_QUERY: &str = r#"
            SELECT id, session_id
            FROM calibration
            WHERE tenant_id = NONE OR tenant_id = '';
            "#;

pub const SELECT_SCOPE_BY_NODE_ID_QUERY: &str = r#"
                        SELECT
                                tenant_id AS TenantId,
                                session_id AS SessionId
                        FROM temporal_node
            WHERE id = type::record('temporal_node', $node_id)
                        LIMIT 1;
                        "#;

pub const COUNT_TEMPORAL_SCOPE_QUERY: &str = r#"
                        SELECT count() AS Count
                        FROM temporal_node
                        WHERE session_id = $session_id
                            AND (tenant_id = $tenant_id OR ($include_legacy AND (tenant_id = NONE OR tenant_id = '')))
                        LIMIT 1;
                        "#;

pub const COUNT_CALIBRATION_SCOPE_QUERY: &str = r#"
                        SELECT count() AS Count
                        FROM calibration
                        WHERE session_id = $session_id
                            AND (tenant_id = $tenant_id OR ($include_legacy AND (tenant_id = NONE OR tenant_id = '')))
                        LIMIT 1;
                        "#;

pub const APPLY_SCOPE_REKEY_QUERY: &str = r#"
                        BEGIN TRANSACTION;

                        UPDATE temporal_node
                        SET
                                tenant_id = $target_tenant_id,
                                session_id = $target_session_id
                        WHERE session_id = $source_session_id
                            AND (tenant_id = $source_tenant_id OR ($source_include_legacy AND (tenant_id = NONE OR tenant_id = '')));

                        UPDATE calibration
                        SET
                                tenant_id = $target_tenant_id,
                                session_id = $target_session_id
                        WHERE session_id = $source_session_id
                            AND (tenant_id = $source_tenant_id OR ($source_include_legacy AND (tenant_id = NONE OR tenant_id = '')));

                        COMMIT TRANSACTION;
                        "#;

pub fn update_record_tenant_query(record_id: &str) -> String {
    format!(
        r#"
            UPDATE {record_id}
            SET tenant_id = $tenant_id;
            "#
    )
}

#[cfg(test)]
mod tests {
    use super::create_temporal_node_query;

    #[test]
    fn create_temporal_node_query_uses_none_for_missing_embedded_at() {
        let query = create_temporal_node_query(
            "abc123", false, false, true, true, false, false, false, false,
        );

        assert!(query.contains("embedded_at = NONE"));
        assert!(!query.contains("embedded_at = <datetime>$embedded_at"));
        assert!(query.contains("embedding = NONE"));
        assert!(query.contains("embedding_model = NONE"));
        assert!(query.contains("embedding_dimensions = NONE"));
    }

    #[test]
    fn create_temporal_node_query_uses_datetime_cast_when_embedded_at_present() {
        let query =
            create_temporal_node_query("abc123", false, false, true, true, true, true, true, true);

        assert!(query.contains("embedded_at = <datetime>$embedded_at"));
    }
}
