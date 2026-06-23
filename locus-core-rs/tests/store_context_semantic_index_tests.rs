use std::sync::Arc;

use locus_core_rs::application::services::StoreContextService;
use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::{NodeStore, SemanticIndexStore, SemanticIndexStoreInitializer};
use locus_core_rs::domain::models::NodeQuery;
use locus_core_rs::parsing::SttpNodeParser;
use locus_core_rs::storage::{InMemoryNodeStore, InMemorySemanticIndexStore};

const TAGGED_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "tag-index-session", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.72, friction: 0.38, logic: 0.81, autonomy: 0.69 }, context_summary: "Tag index sync regression", relevant_tier: raw, retrieval_budget: 5, semantic_tags: ["safety-eval", "instruction-precedence"] } } ⟩
⦿⟨ { timestamp: "2026-04-27T19:00:00Z", tier: raw, session_id: "tag-index-session", user_avec: { stability: 0.72, friction: 0.38, logic: 0.81, autonomy: 0.69, psi: 2.60 }, model_avec: { stability: 0.70, friction: 0.40, logic: 0.82, autonomy: 0.68, psi: 2.60 } } ⟩
◈⟨ { eval_case(.98): "Tag index should use canonical sync key after upsert." } ⟩
⍉⟨ { rho: 0.95, kappa: 0.93, psi: 2.60, compression_avec: { stability: 0.71, friction: 0.39, logic: 0.82, autonomy: 0.68, psi: 2.60 } } ⟩
"#;

#[tokio::test(flavor = "current_thread")]
async fn store_context_syncs_tag_index_with_canonical_sync_key() {
    let store = Arc::new(InMemoryNodeStore::new());
    let index = Arc::new(InMemorySemanticIndexStore::new());
    index.initialize_async().await.expect("index init");

    let service = StoreContextService::new(
        store.clone(),
        Arc::new(TreeSitterValidator),
        SttpNodeParser::new(),
    )
    .with_semantic_index(index.clone());

    let session_id = "tag-index-session";
    let result = service.store_async(TAGGED_NODE, session_id).await;
    assert!(result.valid, "{:?}", result.validation_error);

    let nodes = store
        .query_nodes_async(NodeQuery {
            limit: 10,
            session_id: Some(session_id.to_string()),
            ..Default::default()
        })
        .await
        .expect("query nodes");
    assert_eq!(nodes.len(), 1);
    let stored = &nodes[0];
    assert!(
        !stored.sync_key.trim().is_empty(),
        "store should assign a canonical sync key"
    );

    let indexed = index
        .find_sync_keys_by_tags_async(
            "default",
            &["safety-eval".to_string()],
            false,
            Some(session_id),
            10,
        )
        .await
        .expect("tag lookup");

    assert_eq!(
        indexed,
        vec![stored.sync_key.clone()],
        "indexed_tags pre-filter must resolve the persisted sync key"
    );
}
