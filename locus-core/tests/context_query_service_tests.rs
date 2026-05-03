use std::sync::Arc;

use chrono::{Duration, Utc};
use locus_core::application::services::ContextQueryService;
use locus_core::domain::contracts::{NodeStore, NodeStoreInitializer};
use locus_core::domain::models::{AvecState, SttpNode};
use locus_core::storage::InMemoryNodeStore;

fn make_node(session_id: &str, psi: f32, embedding: Option<Vec<f32>>) -> SttpNode {
    let avec = AvecState {
        stability: 0.85,
        friction: 0.25,
        logic: 0.80,
        autonomy: 0.70,
    };

    SttpNode {
        raw: format!("node-{session_id}-{psi}"),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp: Utc::now(),
        compression_depth: 1,
        parent_node_id: None,
        sync_key: String::new(),
        updated_at: Utc::now(),
        source_metadata: None,
        context_summary: None,
        embedding,
        embedding_model: None,
        embedding_dimensions: None,
        embedded_at: None,
        user_avec: avec,
        model_avec: avec,
        compression_avec: Some(avec),
        rho: 0.95,
        kappa: 0.94,
        psi,
    }
}

fn make_node_with_model_avec(
    session_id: &str,
    psi: f32,
    model_avec: AvecState,
    embedding: Option<Vec<f32>>,
) -> SttpNode {
    let base = AvecState {
        stability: 0.85,
        friction: 0.25,
        logic: 0.80,
        autonomy: 0.70,
    };

    SttpNode {
        raw: format!("node-{session_id}-{psi}"),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp: Utc::now(),
        compression_depth: 1,
        parent_node_id: None,
        sync_key: String::new(),
        updated_at: Utc::now(),
        source_metadata: None,
        context_summary: None,
        embedding,
        embedding_model: None,
        embedding_dimensions: None,
        embedded_at: None,
        user_avec: base,
        model_avec,
        compression_avec: Some(base),
        rho: 0.95,
        kappa: 0.94,
        psi,
    }
}

#[tokio::test]
async fn get_context_global_returns_mixed_sessions() {
    let store = Arc::new(InMemoryNodeStore::new());
    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer
        .initialize_async()
        .await
        .expect("store should initialize");

    store
        .upsert_node_async(make_node("session-a", 2.60, None))
        .await
        .expect("first node should store");
    store
        .upsert_node_async(make_node("session-b", 2.55, None))
        .await
        .expect("second node should store");

    let service = ContextQueryService::new(store);

    let result = service
        .get_context_global_async(0.85, 0.25, 0.80, 0.70, 10)
        .await;

    assert!(result.retrieved >= 2);
    assert!(
        result
            .nodes
            .iter()
            .any(|node| node.session_id == "session-a")
    );
    assert!(
        result
            .nodes
            .iter()
            .any(|node| node.session_id == "session-b")
    );
}

#[tokio::test]
async fn get_context_hybrid_global_prefers_semantic_match() {
    let store = Arc::new(InMemoryNodeStore::new());
    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer
        .initialize_async()
        .await
        .expect("store should initialize");

    store
        .upsert_node_async(make_node("session-a", 2.60, Some(vec![1.0, 0.0, 0.0])))
        .await
        .expect("first node should store");
    store
        .upsert_node_async(make_node("session-b", 2.60, Some(vec![0.0, 1.0, 0.0])))
        .await
        .expect("second node should store");

    let service = ContextQueryService::new(store);

    let result = service
        .get_context_hybrid_global_async(
            0.85,
            0.25,
            0.80,
            0.70,
            Some(&[1.0, 0.0, 0.0]),
            0.3,
            0.7,
            1,
        )
        .await;

    assert_eq!(result.retrieved, 1);
    assert_eq!(result.nodes[0].session_id, "session-a");
}

#[tokio::test]
async fn get_context_scoped_keeps_existing_session_behavior() {
    let store = Arc::new(InMemoryNodeStore::new());
    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer
        .initialize_async()
        .await
        .expect("store should initialize");

    store
        .upsert_node_async(make_node("session-a", 2.60, None))
        .await
        .expect("first node should store");
    store
        .upsert_node_async(make_node("session-b", 2.55, None))
        .await
        .expect("second node should store");

    let service = ContextQueryService::new(store);

    let result = service
        .get_context_scoped_async(Some("session-a"), 0.85, 0.25, 0.80, 0.70, 10)
        .await;

    assert!(result.retrieved >= 1);
    assert!(
        result
            .nodes
            .iter()
            .all(|node| node.session_id == "session-a")
    );
}

#[tokio::test]
async fn get_context_global_prefers_full_avec_match_when_psi_ties() {
    let store = Arc::new(InMemoryNodeStore::new());
    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer
        .initialize_async()
        .await
        .expect("store should initialize");

    let target = AvecState {
        stability: 0.90,
        friction: 0.10,
        logic: 0.90,
        autonomy: 0.70,
    };

    store
        .upsert_node_async(make_node_with_model_avec(
            "session-avec-best",
            2.60,
            target,
            None,
        ))
        .await
        .expect("best match node should store");

    store
        .upsert_node_async(make_node_with_model_avec(
            "session-avec-worse",
            2.60,
            AvecState {
                stability: 0.90,
                friction: 0.90,
                logic: 0.10,
                autonomy: 0.70,
            },
            None,
        ))
        .await
        .expect("worse match node should store");

    let service = ContextQueryService::new(store);

    let result = service
        .get_context_global_async(
            target.stability,
            target.friction,
            target.logic,
            target.autonomy,
            1,
        )
        .await;

    assert_eq!(result.retrieved, 1);
    assert_eq!(result.nodes[0].session_id, "session-avec-best");
}

#[tokio::test]
async fn get_context_scoped_filtered_applies_time_window_and_tiers() {
    let store = Arc::new(InMemoryNodeStore::new());
    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer
        .initialize_async()
        .await
        .expect("store should initialize");

    let now = Utc::now();

    let mut keep = make_node("session-filter", 2.60, None);
    keep.tier = "weekly".to_string();
    keep.timestamp = now - Duration::hours(2);

    let mut wrong_tier = make_node("session-filter", 2.60, None);
    wrong_tier.tier = "raw".to_string();
    wrong_tier.timestamp = now - Duration::hours(2);

    let mut wrong_time = make_node("session-filter", 2.60, None);
    wrong_time.tier = "weekly".to_string();
    wrong_time.timestamp = now - Duration::days(8);

    store
        .upsert_node_async(keep)
        .await
        .expect("keep node should store");
    store
        .upsert_node_async(wrong_tier)
        .await
        .expect("wrong tier node should store");
    store
        .upsert_node_async(wrong_time)
        .await
        .expect("wrong time node should store");

    let service = ContextQueryService::new(store);
    let tiers = vec!["weekly".to_string()];
    let result = service
        .get_context_scoped_filtered_async(
            Some("session-filter"),
            0.85,
            0.25,
            0.80,
            0.70,
            Some(now - Duration::days(7)),
            Some(now),
            Some(&tiers),
            10,
        )
        .await;

    assert_eq!(result.retrieved, 1);
    assert_eq!(result.nodes[0].tier, "weekly");
}
