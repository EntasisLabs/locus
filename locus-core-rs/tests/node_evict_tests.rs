use chrono::{DateTime, Utc};
use locus_core_rs::domain::contracts::NodeStore;
use locus_core_rs::domain::models::{
    AvecState, NodeDeleteStatus, NodeQuery, NodeUpsertStatus, SessionPurgeRequest, SttpNode,
};
use locus_core_rs::storage::InMemoryNodeStore;

fn build_test_node(session_id: &str, sync_key: &str) -> SttpNode {
    SttpNode {
        raw: "raw".to_string(),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp: DateTime::parse_from_rfc3339("2026-03-05T06:30:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc),
        compression_depth: 1,
        parent_node_id: None,
        sync_key: sync_key.to_string(),
        updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:30:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc),
        source_metadata: None,
        context_summary: None,
        semantic_tags: None,
        semantic_links: None,
        embedding: None,
        embedding_model: None,
        embedding_dimensions: None,
        embedded_at: None,
        user_avec: AvecState::zero(),
        model_avec: AvecState::zero(),
        compression_avec: None,
        rho: 0.9,
        kappa: 0.9,
        psi: 2.0,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn delete_node_by_sync_key_removes_row() {
    let store = InMemoryNodeStore::new();
    let session = "evict-session";
    store
        .upsert_node_async(build_test_node(session, "sync-42"))
        .await
        .expect("upsert");

    let result = store
        .delete_nodes_async(locus_core_rs::domain::models::NodeDeleteRequest {
            tenant_id: "default".to_string(),
            session_id: session.to_string(),
            sync_keys: vec!["sync-42".to_string()],
            node_ids: vec![],
            dry_run: false,
        })
        .await
        .expect("delete");

    assert_eq!(result.deleted, 1);
    assert_eq!(result.records[0].status, NodeDeleteStatus::Deleted);

    let remaining = store
        .query_nodes_async(NodeQuery {
            limit: 10,
            session_id: Some(session.to_string()),
            ..Default::default()
        })
        .await
        .expect("query");
    assert!(remaining.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn dry_run_delete_does_not_remove_rows() {
    let store = InMemoryNodeStore::new();
    let session = "evict-session";
    store
        .upsert_node_async(build_test_node(session, "sync-42"))
        .await
        .expect("upsert");

    let result = store
        .delete_nodes_async(locus_core_rs::domain::models::NodeDeleteRequest {
            tenant_id: "default".to_string(),
            session_id: session.to_string(),
            sync_keys: vec!["sync-42".to_string()],
            node_ids: vec![],
            dry_run: true,
        })
        .await
        .expect("delete");

    assert_eq!(result.deleted, 1);
    assert_eq!(result.records[0].status, NodeDeleteStatus::Skipped);

    let remaining = store
        .query_nodes_async(NodeQuery {
            limit: 10,
            session_id: Some(session.to_string()),
            ..Default::default()
        })
        .await
        .expect("query");
    assert_eq!(remaining.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn delete_by_node_id_removes_row() {
    let store = InMemoryNodeStore::new();
    let session = "evict-session";
    let upsert = store
        .upsert_node_async(build_test_node(session, "sync-42"))
        .await
        .expect("upsert");
    assert_eq!(upsert.status, NodeUpsertStatus::Created);

    let result = store
        .delete_nodes_async(locus_core_rs::domain::models::NodeDeleteRequest {
            tenant_id: "default".to_string(),
            session_id: session.to_string(),
            sync_keys: vec![],
            node_ids: vec![upsert.node_id],
            dry_run: false,
        })
        .await
        .expect("delete");

    assert_eq!(result.deleted, 1);
}

#[tokio::test(flavor = "current_thread")]
async fn session_purge_removes_matching_nodes() {
    let store = InMemoryNodeStore::new();
    let session = "purge-session";
    store
        .upsert_node_async(build_test_node(session, "one"))
        .await
        .expect("upsert one");
    store
        .upsert_node_async(build_test_node(session, "two"))
        .await
        .expect("upsert two");

    let result = store
        .purge_session_async(SessionPurgeRequest {
            tenant_id: "default".to_string(),
            session_id: session.to_string(),
            tiers: None,
            dry_run: false,
            include_calibration: false,
            include_checkpoints: false,
        })
        .await
        .expect("purge");

    assert_eq!(result.deleted, 2);
}
