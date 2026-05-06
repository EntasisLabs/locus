use chrono::{DateTime, Utc};
use locus_core_rs::domain::contracts::NodeStore;
use locus_core_rs::domain::models::{
    AvecState, ConnectorMetadata, NodeUpsertStatus, SttpNode, SyncCheckpoint, SyncCursor,
};
use locus_core_rs::storage::InMemoryNodeStore;

fn build_test_node(session_id: &str) -> SttpNode {
    SttpNode {
        raw: "raw".to_string(),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp: DateTime::parse_from_rfc3339("2026-03-05T06:30:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc),
        compression_depth: 1,
        parent_node_id: None,
        sync_key: String::new(),
        updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:30:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc),
        source_metadata: None,
        context_summary: None,
        embedding: None,
        embedding_model: None,
        embedding_dimensions: None,
        embedded_at: None,
        user_avec: AvecState {
            stability: 0.85,
            friction: 0.25,
            logic: 0.80,
            autonomy: 0.70,
        },
        model_avec: AvecState {
            stability: 0.91,
            friction: 0.21,
            logic: 0.90,
            autonomy: 0.80,
        },
        compression_avec: Some(AvecState::zero()),
        rho: 0.96,
        kappa: 0.94,
        psi: 2.6,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn duplicate_upsert_does_not_create_extra_rows() {
    let store = InMemoryNodeStore::new();

    let first = store
        .upsert_node_async(build_test_node("sync-session"))
        .await
        .expect("first upsert should succeed");
    let second = store
        .upsert_node_async(build_test_node("sync-session"))
        .await
        .expect("second upsert should succeed");

    assert_eq!(first.status, NodeUpsertStatus::Created);
    assert_eq!(second.status, NodeUpsertStatus::Duplicate);

    let nodes = store
        .query_nodes_async(locus_core_rs::domain::models::NodeQuery {
            limit: 10,
            session_id: Some("sync-session".to_string()),
            from_utc: None,
            to_utc: None,
            tiers: None,
        })
        .await
        .expect("query should succeed");
    assert_eq!(nodes.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn change_query_returns_incremental_cursor() {
    let store = InMemoryNodeStore::new();

    let mut first = build_test_node("sync-session");
    first.updated_at = DateTime::parse_from_rfc3339("2026-03-05T06:31:00Z")
        .expect("timestamp should parse")
        .with_timezone(&Utc);
    first.sync_key = "sync-a".to_string();

    let mut second = build_test_node("sync-session");
    second.raw = "raw-b".to_string();
    second.timestamp = DateTime::parse_from_rfc3339("2026-03-05T06:32:00Z")
        .expect("timestamp should parse")
        .with_timezone(&Utc);
    second.updated_at = DateTime::parse_from_rfc3339("2026-03-05T06:33:00Z")
        .expect("timestamp should parse")
        .with_timezone(&Utc);
    second.sync_key = "sync-b".to_string();

    let first_result = store
        .upsert_node_async(first)
        .await
        .expect("first upsert should succeed");
    let second_result = store
        .upsert_node_async(second)
        .await
        .expect("second upsert should succeed");

    let result = store
        .query_changes_since_async(
            "sync-session",
            Some(SyncCursor {
                updated_at: first_result.updated_at,
                sync_key: first_result.sync_key,
            }),
            10,
        )
        .await
        .expect("change query should succeed");

    assert_eq!(result.nodes.len(), 1);
    assert_eq!(result.nodes[0].sync_key, second_result.sync_key);
}

#[tokio::test(flavor = "current_thread")]
async fn checkpoints_replace_existing_connector_state() {
    let store = InMemoryNodeStore::new();

    store
        .put_checkpoint_async(SyncCheckpoint {
            session_id: "sync-session".to_string(),
            connector_id: "cloud-primary".to_string(),
            cursor: Some(SyncCursor {
                updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:35:00Z")
                    .expect("timestamp should parse")
                    .with_timezone(&Utc),
                sync_key: "sync-a".to_string(),
            }),
            updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:36:00Z")
                .expect("timestamp should parse")
                .with_timezone(&Utc),
            metadata: Some(ConnectorMetadata {
                connector_id: "cloud-primary".to_string(),
                source_kind: "local".to_string(),
                upstream_id: "node-a".to_string(),
                revision: Some("1".to_string()),
                observed_at_utc: DateTime::parse_from_rfc3339("2026-03-05T06:36:00Z")
                    .expect("timestamp should parse")
                    .with_timezone(&Utc),
                extra: Some(serde_json::json!({ "endpoint": "local" })),
            }),
        })
        .await
        .expect("checkpoint insert should succeed");

    store
        .put_checkpoint_async(SyncCheckpoint {
            session_id: "sync-session".to_string(),
            connector_id: "cloud-primary".to_string(),
            cursor: Some(SyncCursor {
                updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:40:00Z")
                    .expect("timestamp should parse")
                    .with_timezone(&Utc),
                sync_key: "sync-b".to_string(),
            }),
            updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:41:00Z")
                .expect("timestamp should parse")
                .with_timezone(&Utc),
            metadata: Some(ConnectorMetadata {
                connector_id: "cloud-primary".to_string(),
                source_kind: "cloud".to_string(),
                upstream_id: "node-b".to_string(),
                revision: Some("2".to_string()),
                observed_at_utc: DateTime::parse_from_rfc3339("2026-03-05T06:41:00Z")
                    .expect("timestamp should parse")
                    .with_timezone(&Utc),
                extra: Some(serde_json::json!({ "endpoint": "cloud" })),
            }),
        })
        .await
        .expect("checkpoint update should succeed");

    let checkpoint = store
        .get_checkpoint_async("sync-session", "cloud-primary")
        .await
        .expect("checkpoint query should succeed")
        .expect("checkpoint should exist");

    assert_eq!(
        checkpoint
            .cursor
            .as_ref()
            .map(|cursor| cursor.sync_key.as_str()),
        Some("sync-b")
    );
    assert_eq!(
        checkpoint
            .metadata
            .as_ref()
            .map(|metadata| metadata.source_kind.as_str()),
        Some("cloud")
    );
    assert_eq!(
        checkpoint
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.revision.as_deref()),
        Some("2")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hybrid_query_prefers_semantic_match_with_fallback_for_missing_vectors() {
    let store = InMemoryNodeStore::new();

    let mut left = build_test_node("sync-session");
    left.sync_key = "sync-left".to_string();
    left.embedding = Some(vec![1.0, 0.0, 0.0]);
    left.embedding_dimensions = Some(3);
    left.embedding_model = Some("test-model".to_string());
    left.context_summary = Some("left".to_string());

    let mut right = build_test_node("sync-session");
    right.sync_key = "sync-right".to_string();
    right.embedding = Some(vec![0.0, 1.0, 0.0]);
    right.embedding_dimensions = Some(3);
    right.embedding_model = Some("test-model".to_string());
    right.context_summary = Some("right".to_string());

    let mut fallback = build_test_node("sync-session");
    fallback.sync_key = "sync-fallback".to_string();
    fallback.embedding = None;
    fallback.context_summary = Some("fallback".to_string());

    store
        .upsert_node_async(left)
        .await
        .expect("left upsert should succeed");
    store
        .upsert_node_async(right)
        .await
        .expect("right upsert should succeed");
    store
        .upsert_node_async(fallback)
        .await
        .expect("fallback upsert should succeed");

    let result = store
        .get_by_hybrid_async(
            "sync-session",
            AvecState {
                stability: 0.85,
                friction: 0.25,
                logic: 0.80,
                autonomy: 0.70,
            },
            None,
            None,
            None,
            Some(&[0.0, 1.0, 0.0]),
            0.5,
            0.5,
            3,
        )
        .await
        .expect("hybrid query should succeed");

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].sync_key, "sync-right");
    assert!(result.iter().any(|node| node.sync_key == "sync-fallback"));
}
