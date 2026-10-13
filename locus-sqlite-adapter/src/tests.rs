use std::sync::Arc;

use chrono::{TimeZone, Utc};
use locus_core_rs::domain::contracts::{
    NodeStore, NodeStoreInitializer, SemanticIndexStore, SemanticIndexStoreInitializer,
};
use locus_core_rs::domain::models::{
    AvecState, NodeDeleteRequest, NodeQuery, NodeUpsertStatus, SemanticTagNodeRef,
    SemanticTagQueryFilter, SessionPurgeRequest, SttpNode, SyncCheckpoint, SyncCursor,
};

use crate::{SqliteDatabase, SqliteNodeStore, SqliteSemanticIndexStore};

fn avec(stability: f32) -> AvecState {
    AvecState {
        stability,
        friction: 0.2,
        logic: 0.5,
        autonomy: 0.4,
    }
}

fn node(session_id: &str, sync_key: &str, stability: f32) -> SttpNode {
    let avec = avec(stability);
    SttpNode {
        raw: format!("raw-{sync_key}"),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp: Utc.with_ymd_and_hms(2026, 3, 5, 6, 30, 0).unwrap(),
        compression_depth: 1,
        parent_node_id: None,
        sync_key: sync_key.to_string(),
        updated_at: Utc.with_ymd_and_hms(2026, 3, 5, 6, 30, 0).unwrap(),
        source_metadata: None,
        context_summary: Some(format!("summary-{sync_key}")),
        semantic_tags: Some(vec!["alpha".to_string()]),
        semantic_links: None,
        embedding: None,
        embedding_model: None,
        embedding_dimensions: None,
        embedded_at: None,
        user_avec: avec,
        model_avec: avec,
        compression_avec: Some(avec),
        rho: 0.9,
        kappa: 0.8,
        psi: avec.psi(),
    }
}

async fn open_stores() -> (Arc<SqliteNodeStore>, Arc<SqliteSemanticIndexStore>) {
    let db = Arc::new(SqliteDatabase::open_in_memory().expect("memory db"));
    let nodes = Arc::new(SqliteNodeStore::new(db.clone()));
    let tags = Arc::new(SqliteSemanticIndexStore::new(db));
    NodeStoreInitializer::initialize_async(nodes.as_ref())
        .await
        .expect("node schema");
    SemanticIndexStoreInitializer::initialize_async(tags.as_ref())
        .await
        .expect("tag schema");
    (nodes, tags)
}

#[tokio::test]
async fn upsert_classifies_create_duplicate_and_update() {
    let (nodes, _) = open_stores().await;
    let created = nodes
        .upsert_node_async(node("demo", "sync-1", 0.8))
        .await
        .expect("create");
    assert_eq!(created.status, NodeUpsertStatus::Created);

    let duplicate = nodes
        .upsert_node_async(node("demo", "sync-1", 0.8))
        .await
        .expect("duplicate");
    assert_eq!(duplicate.status, NodeUpsertStatus::Duplicate);
    assert_eq!(duplicate.node_id, created.node_id);

    let mut changed = node("demo", "sync-1", 0.8);
    changed.raw = "raw-changed".to_string();
    let updated = nodes.upsert_node_async(changed).await.expect("update");
    assert_eq!(updated.status, NodeUpsertStatus::Updated);
    assert_eq!(updated.node_id, created.node_id);

    let stored = nodes
        .query_nodes_async(NodeQuery {
            session_id: Some("demo".to_string()),
            limit: 10,
            ..NodeQuery::default()
        })
        .await
        .expect("query");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].raw, "raw-changed");
}

#[tokio::test]
async fn resonance_orders_by_model_avec_distance() {
    let (nodes, _) = open_stores().await;
    nodes
        .upsert_node_async(node("demo", "far", 0.1))
        .await
        .expect("far");
    nodes
        .upsert_node_async(node("demo", "near", 0.8))
        .await
        .expect("near");

    let ranked = nodes
        .get_by_resonance_async("demo", avec(0.8), None, None, None, 2)
        .await
        .expect("resonance");
    assert_eq!(ranked[0].sync_key, "near");
    assert_eq!(ranked[1].sync_key, "far");
}

#[tokio::test]
async fn hybrid_prefers_embedding_match_inside_candidate_window() {
    let (nodes, _) = open_stores().await;
    let mut close_avec = node("demo", "avec", 0.8);
    close_avec.embedding = Some(vec![0.0, 1.0]);
    let mut close_semantic = node("demo", "semantic", 0.55);
    close_semantic.embedding = Some(vec![1.0, 0.0]);
    nodes.upsert_node_async(close_avec).await.expect("avec");
    nodes
        .upsert_node_async(close_semantic)
        .await
        .expect("semantic");

    let ranked = nodes
        .get_by_hybrid_async(
            "demo",
            avec(0.8),
            None,
            None,
            None,
            Some(&[1.0, 0.0]),
            0.2,
            0.8,
            1,
        )
        .await
        .expect("hybrid");
    assert_eq!(ranked[0].sync_key, "semantic");
}

#[tokio::test]
async fn changes_cursor_pages_in_sync_key_order() {
    let (nodes, _) = open_stores().await;
    let mut first = node("demo", "a", 0.5);
    let mut second = node("demo", "b", 0.5);
    first.updated_at = Utc.with_ymd_and_hms(2026, 3, 5, 1, 0, 0).unwrap();
    second.updated_at = Utc.with_ymd_and_hms(2026, 3, 5, 2, 0, 0).unwrap();
    nodes.upsert_node_async(first).await.expect("first");
    nodes.upsert_node_async(second).await.expect("second");

    let page = nodes
        .query_changes_since_async("demo", None, 1)
        .await
        .expect("page");
    assert!(page.has_more);
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(page.nodes[0].sync_key, "a");

    let next = nodes
        .query_changes_since_async("demo", page.next_cursor, 10)
        .await
        .expect("next");
    assert!(!next.has_more);
    assert_eq!(next.nodes[0].sync_key, "b");
}

#[tokio::test]
async fn checkpoint_round_trip_and_calibration_history() {
    let (nodes, _) = open_stores().await;
    nodes
        .store_calibration_async("demo", avec(0.2), "first")
        .await
        .expect("first calibration");
    nodes
        .store_calibration_async("demo", avec(0.9), "second")
        .await
        .expect("second calibration");

    let last = nodes
        .get_last_avec_async("demo")
        .await
        .expect("last")
        .unwrap();
    assert!((last.stability - 0.9).abs() < 0.0001);
    let history = nodes
        .get_trigger_history_async("demo")
        .await
        .expect("history");
    assert_eq!(history, vec!["first".to_string(), "second".to_string()]);

    let checkpoint = SyncCheckpoint {
        session_id: "demo".to_string(),
        connector_id: "connector".to_string(),
        cursor: Some(SyncCursor {
            updated_at: Utc.with_ymd_and_hms(2026, 3, 5, 6, 0, 0).unwrap(),
            sync_key: "a".to_string(),
        }),
        updated_at: Utc.with_ymd_and_hms(2026, 3, 6, 0, 0, 0).unwrap(),
        metadata: None,
    };
    nodes
        .put_checkpoint_async(checkpoint)
        .await
        .expect("put checkpoint");
    let stored = nodes
        .get_checkpoint_async("demo", "connector")
        .await
        .expect("get checkpoint")
        .expect("checkpoint exists");
    assert_eq!(stored.cursor.unwrap().sync_key, "a");
}

#[tokio::test]
async fn delete_dry_run_then_purge_removes_tags() {
    let (nodes, tags) = open_stores().await;
    let created = nodes
        .upsert_node_async(node("demo", "sync-1", 0.4))
        .await
        .expect("create");
    tags.sync_node_tags_async(
        SemanticTagNodeRef {
            tenant_id: "default".to_string(),
            session_id: "demo".to_string(),
            node_id: created.node_id.clone(),
            sync_key: "sync-1".to_string(),
        },
        &["alpha".to_string(), "beta".to_string()],
        None,
    )
    .await
    .expect("tags");

    let preview = nodes
        .delete_nodes_async(NodeDeleteRequest {
            tenant_id: "default".to_string(),
            session_id: "demo".to_string(),
            sync_keys: vec!["sync-1".to_string()],
            dry_run: true,
            ..NodeDeleteRequest::default()
        })
        .await
        .expect("preview");
    assert_eq!(preview.deleted, 1);
    assert_eq!(preview.records[0].reason.as_deref(), Some("would delete"));

    let purged = nodes
        .purge_session_async(SessionPurgeRequest {
            tenant_id: "default".to_string(),
            session_id: "demo".to_string(),
            include_calibration: true,
            ..SessionPurgeRequest::default()
        })
        .await
        .expect("purge");
    assert_eq!(purged.deleted, 1);

    let remaining = tags
        .query_tag_records_async(SemanticTagQueryFilter {
            tenant_id: Some("default".to_string()),
            limit: 10,
            ..SemanticTagQueryFilter::default()
        })
        .await
        .expect("tags after purge");
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn rekey_moves_session_scope() {
    let (nodes, _) = open_stores().await;
    let created = nodes
        .upsert_node_async(node("demo", "sync-1", 0.4))
        .await
        .expect("create");
    nodes
        .store_calibration_async("demo", avec(0.4), "seed")
        .await
        .expect("calibration");

    let moved = nodes
        .batch_rekey_scopes_async(
            vec![format!("temporal_node:{}", created.node_id)],
            "acme",
            "tenant:acme::session:next",
            false,
            false,
        )
        .await
        .expect("rekey");
    assert_eq!(moved.temporal_nodes_updated, 1);
    assert_eq!(moved.calibrations_updated, 1);

    let source = nodes
        .query_nodes_async(NodeQuery {
            session_id: Some("demo".to_string()),
            limit: 10,
            ..NodeQuery::default()
        })
        .await
        .expect("source");
    assert!(source.is_empty());

    let target = nodes
        .query_nodes_async(NodeQuery {
            session_id: Some("tenant:acme::session:next".to_string()),
            limit: 10,
            ..NodeQuery::default()
        })
        .await
        .expect("target");
    assert_eq!(target.len(), 1);
    assert_eq!(target[0].sync_key, "sync-1");
}

#[tokio::test]
async fn tag_sync_replaces_removed_tags_and_matches_all() {
    let (_, tags) = open_stores().await;
    for (sync_key, labels) in [("a", vec!["x", "y"]), ("b", vec!["x"])] {
        tags.sync_node_tags_async(
            SemanticTagNodeRef {
                tenant_id: "default".to_string(),
                session_id: "demo".to_string(),
                node_id: sync_key.to_string(),
                sync_key: sync_key.to_string(),
            },
            &labels.into_iter().map(str::to_string).collect::<Vec<_>>(),
            None,
        )
        .await
        .expect("sync");
    }

    tags.sync_node_tags_async(
        SemanticTagNodeRef {
            tenant_id: "default".to_string(),
            session_id: "demo".to_string(),
            node_id: "a".to_string(),
            sync_key: "a".to_string(),
        },
        &["x".to_string()],
        None,
    )
    .await
    .expect("resync");

    let keys = tags
        .find_sync_keys_by_tags_async(
            "default",
            &["x".to_string(), "y".to_string()],
            true,
            None,
            10,
        )
        .await
        .expect("match all");
    assert!(keys.is_empty());

    let any = tags
        .find_sync_keys_by_tags_async("default", &["x".to_string()], false, None, 10)
        .await
        .expect("any");
    assert_eq!(any, vec!["a".to_string(), "b".to_string()]);
}
