use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use locus_core::domain::contracts::{NodeStore, NodeStoreInitializer};
use locus_core::domain::models::{
    AvecState, ConnectorMetadata, NodeQuery, NodeUpsertStatus, SttpNode, SyncCheckpoint, SyncCursor,
};
use locus_core::storage::surrealdb::{QueryParams, SurrealDbClient, SurrealDbNodeStore};

#[derive(Default)]
struct MockSurrealDbClient {
    responses: Mutex<VecDeque<Vec<Value>>>,
    queries: Mutex<Vec<String>>,
    parameters: Mutex<Vec<QueryParams>>,
}

impl MockSurrealDbClient {
    async fn queue_response(&self, rows: Vec<Value>) {
        self.responses.lock().await.push_back(rows);
    }

    async fn queries(&self) -> Vec<String> {
        self.queries.lock().await.clone()
    }

    async fn parameters(&self) -> Vec<QueryParams> {
        self.parameters.lock().await.clone()
    }
}

#[async_trait]
impl SurrealDbClient for MockSurrealDbClient {
    async fn raw_query(&self, query: &str, parameters: QueryParams) -> Result<Vec<Value>> {
        self.queries.lock().await.push(query.to_string());
        self.parameters.lock().await.push(parameters);
        Ok(self.responses.lock().await.pop_front().unwrap_or_default())
    }
}

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
async fn initialize_runs_schema_query() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    store
        .initialize_async()
        .await
        .expect("schema initialization should succeed");

    let queries = client.queries().await;
    assert_eq!(queries.len(), 3);
    assert!(queries[0].contains("DEFINE TABLE IF NOT EXISTS temporal_node"));
    assert!(queries[0].contains("DEFINE FIELD IF NOT EXISTS tenant_id"));
    assert!(queries[1].contains("FROM temporal_node"));
    assert!(queries[2].contains("FROM calibration"));
}

#[tokio::test(flavor = "current_thread")]
async fn initialize_backfills_tenant_ids_for_legacy_rows() {
    let client = Arc::new(MockSurrealDbClient::default());

    client.queue_response(vec![]).await;

    client
        .queue_response(vec![json!({
            "id": "temporal_node:legacy_node",
            "session_id": "tenant:acme::session:alpha"
        })])
        .await;

    client.queue_response(vec![]).await;

    client
        .queue_response(vec![json!({
            "id": "calibration:legacy_cal",
            "session_id": "legacy-session"
        })])
        .await;

    client.queue_response(vec![]).await;

    let store = SurrealDbNodeStore::new(client.clone());
    store
        .initialize_async()
        .await
        .expect("schema initialization should succeed");

    let queries = client.queries().await;
    assert!(
        queries
            .iter()
            .any(|query| query.contains("UPDATE temporal_node:`legacy_node`"))
    );
    assert!(
        queries
            .iter()
            .any(|query| query.contains("UPDATE calibration:legacy_cal"))
    );

    let params = client.parameters().await;
    assert!(
        params
            .iter()
            .any(|param| param.get("tenant_id") == Some(&json!("acme")))
    );
    assert!(
        params
            .iter()
            .any(|param| param.get("tenant_id") == Some(&json!("default")))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn initialize_backfills_missing_temporal_sync_fields() {
    let client = Arc::new(MockSurrealDbClient::default());

    client.queue_response(vec![]).await;

    client
        .queue_response(vec![json!({
            "id": "temporal_node:legacy_sync",
            "session_id": "tenant:acme::session:alpha",
            "timestamp": "2026-03-05T06:30:00Z",
            "sync_key": null,
            "updated_at": null
        })])
        .await;

    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;

    let store = SurrealDbNodeStore::new(client.clone());
    store
        .initialize_async()
        .await
        .expect("schema initialization should succeed");

    let queries = client.queries().await;
    assert!(
        queries
            .iter()
            .any(|query| query.contains("UPDATE temporal_node:`legacy_sync`"))
    );

    let params = client.parameters().await;
    let temporal_params = params
        .iter()
        .find(|param| param.get("sync_key").is_some() && param.get("updated_at").is_some())
        .expect("temporal backfill update should include sync_key and updated_at");

    assert_eq!(temporal_params.get("tenant_id"), Some(&json!("acme")));
    assert_eq!(
        temporal_params.get("sync_key"),
        Some(&json!("legacy:legacy_sync"))
    );

    let updated_at = temporal_params
        .get("updated_at")
        .and_then(Value::as_str)
        .expect("updated_at should be serialized as RFC3339 string");
    let parsed_updated_at = DateTime::parse_from_rfc3339(updated_at)
        .expect("updated_at should parse")
        .with_timezone(&Utc);

    assert_eq!(
        parsed_updated_at,
        DateTime::parse_from_rfc3339("2026-03-05T06:30:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn query_nodes_maps_result_rows_to_domain_nodes() {
    let client = Arc::new(MockSurrealDbClient::default());
    client
        .queue_response(vec![json!({
            "SessionId": "s1",
            "Raw": "node raw",
            "Tier": "raw",
            "Timestamp": "2026-03-05T06:30:00Z",
            "CompressionDepth": 1,
            "ParentNodeId": null,
            "Psi": 2.6,
            "Rho": 0.96,
            "Kappa": 0.94,
            "UserStability": 0.85,
            "UserFriction": 0.25,
            "UserLogic": 0.80,
            "UserAutonomy": 0.70,
            "UserPsi": 2.60,
            "ModelStability": 0.85,
            "ModelFriction": 0.25,
            "ModelLogic": 0.80,
            "ModelAutonomy": 0.70,
            "ModelPsi": 2.60,
            "CompStability": 0.85,
            "CompFriction": 0.25,
            "CompLogic": 0.80,
            "CompAutonomy": 0.70,
            "CompPsi": 2.60,
            "ResonanceDelta": 0.0
        })])
        .await;

    let store = SurrealDbNodeStore::new(client.clone());

    let nodes = store
        .query_nodes_async(NodeQuery {
            limit: 5,
            session_id: Some("s1".to_string()),
            from_utc: None,
            to_utc: None,
            tiers: None,
        })
        .await
        .expect("query should succeed");

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].session_id, "s1");
    assert_eq!(nodes[0].tier, "raw");
    assert!((nodes[0].rho - 0.96).abs() <= 0.0001);

    let queries = client.queries().await;
    assert_eq!(queries.len(), 1);
    assert!(queries[0].contains("tenant_id = $tenant_id OR tenant_id = NONE OR tenant_id = ''"));
    assert!(queries[0].contains("session_id = $session_id"));

    let params = client.parameters().await;
    assert_eq!(params.len(), 1);
    assert_eq!(
        params[0]
            .get("tenant_id")
            .expect("tenant_id should be present for session-scoped query"),
        &json!("default")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn store_uses_model_avec_when_compression_avec_is_zero() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;

    let node = build_test_node("session");

    let node_id = store.store_async(node).await.expect("store should succeed");
    assert!(!node_id.trim().is_empty());

    let params = client.parameters().await;
    assert_eq!(params.len(), 3);

    let comp_stability = params[2]
        .get("comp_stability")
        .expect("comp_stability must be present")
        .as_f64()
        .expect("comp_stability must be numeric");
    assert!((comp_stability - 0.91).abs() <= 0.0001);
    assert_eq!(
        params[2]
            .get("tenant_id")
            .expect("tenant_id must be present"),
        &json!("default")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn store_derives_tenant_id_from_scoped_session_key() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;

    let node = build_test_node("tenant:acme::session:session-42");

    store.store_async(node).await.expect("store should succeed");

    let params = client.parameters().await;
    assert_eq!(params.len(), 3);
    assert_eq!(
        params[2]
            .get("tenant_id")
            .expect("tenant_id must be present"),
        &json!("acme")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn upsert_returns_duplicate_when_sync_identity_already_exists() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    // First upsert: no existing exact/any match, then create.
    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;
    client.queue_response(vec![]).await;

    // Second upsert: exact match is found and treated as duplicate.
    client.queue_response(vec![]).await;
    client
        .queue_response(vec![json!({
            "Id": "temporal_node:existing-node",
            "SourceMetadata": null
        })])
        .await;

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
    assert_eq!(second.node_id, "existing-node");

    let queries = client.queries().await;
    assert_eq!(queries.len(), 5);
    assert!(queries[0].contains("sync_key = $sync_key"));
    assert!(queries[1].contains("session_id = $session_id"));
    assert!(queries[2].contains("sync_key = $sync_key"));
    assert!(queries[3].contains("sync_key = $sync_key"));
    assert!(queries[4].contains("sync_key = $sync_key"));
}

#[tokio::test(flavor = "current_thread")]
async fn query_changes_since_returns_incremental_cursor() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    client
        .queue_response(vec![
            json!({
                "SessionId": "sync-session",
                "Raw": "node-a",
                "Tier": "raw",
                "Timestamp": "2026-03-05T06:30:00Z",
                "CompressionDepth": 1,
                "ParentNodeId": null,
                "SyncKey": "sync-a",
                "UpdatedAt": "2026-03-05T06:31:00Z",
                "SourceMetadata": null,
                "Psi": 2.6,
                "Rho": 0.96,
                "Kappa": 0.94,
                "UserStability": 0.85,
                "UserFriction": 0.25,
                "UserLogic": 0.80,
                "UserAutonomy": 0.70,
                "UserPsi": 2.60,
                "ModelStability": 0.85,
                "ModelFriction": 0.25,
                "ModelLogic": 0.80,
                "ModelAutonomy": 0.70,
                "ModelPsi": 2.60,
                "CompStability": 0.85,
                "CompFriction": 0.25,
                "CompLogic": 0.80,
                "CompAutonomy": 0.70,
                "CompPsi": 2.60,
                "ResonanceDelta": 0.0
            }),
            json!({
                "SessionId": "sync-session",
                "Raw": "node-b",
                "Tier": "raw",
                "Timestamp": "2026-03-05T06:32:00Z",
                "CompressionDepth": 1,
                "ParentNodeId": null,
                "SyncKey": "sync-b",
                "UpdatedAt": "2026-03-05T06:33:00Z",
                "SourceMetadata": null,
                "Psi": 2.6,
                "Rho": 0.96,
                "Kappa": 0.94,
                "UserStability": 0.85,
                "UserFriction": 0.25,
                "UserLogic": 0.80,
                "UserAutonomy": 0.70,
                "UserPsi": 2.60,
                "ModelStability": 0.85,
                "ModelFriction": 0.25,
                "ModelLogic": 0.80,
                "ModelAutonomy": 0.70,
                "ModelPsi": 2.60,
                "CompStability": 0.85,
                "CompFriction": 0.25,
                "CompLogic": 0.80,
                "CompAutonomy": 0.70,
                "CompPsi": 2.60,
                "ResonanceDelta": 0.0
            }),
        ])
        .await;

    let result = store
        .query_changes_since_async("sync-session", None, 1)
        .await
        .expect("change query should succeed");

    assert_eq!(result.nodes.len(), 1);
    assert!(result.has_more);
    assert_eq!(result.nodes[0].sync_key, "sync-a");
    assert_eq!(
        result
            .next_cursor
            .as_ref()
            .map(|cursor| cursor.sync_key.as_str()),
        Some("sync-a")
    );

    let params = client.parameters().await;
    assert_eq!(params[0].get("include_cursor"), Some(&json!(false)));
}

#[tokio::test(flavor = "current_thread")]
async fn checkpoints_can_be_read_and_written() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    client
        .queue_response(vec![json!({
            "SessionId": "sync-session",
            "ConnectorId": "cloud-primary",
            "CursorUpdatedAt": "2026-03-05T06:35:00Z",
            "CursorSyncKey": "sync-b",
            "UpdatedAt": "2026-03-05T06:36:00Z",
            "Metadata": {
                "connectorId": "cloud-primary",
                "sourceKind": "cloud",
                "upstreamId": "checkpoint/sync-session",
                "revision": "r1",
                "observedAtUtc": "2026-03-05T06:36:00Z",
                "extra": { "endpoint": "cloud" }
            }
        })])
        .await;
    client.queue_response(vec![]).await;

    let checkpoint = store
        .get_checkpoint_async("sync-session", "cloud-primary")
        .await
        .expect("checkpoint query should succeed")
        .expect("checkpoint should exist");
    assert_eq!(checkpoint.connector_id, "cloud-primary");
    assert_eq!(
        checkpoint
            .cursor
            .as_ref()
            .map(|cursor| cursor.sync_key.as_str()),
        Some("sync-b")
    );

    store
        .put_checkpoint_async(SyncCheckpoint {
            session_id: "sync-session".to_string(),
            connector_id: "cloud-primary".to_string(),
            cursor: Some(SyncCursor {
                updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:40:00Z")
                    .expect("timestamp should parse")
                    .with_timezone(&Utc),
                sync_key: "sync-c".to_string(),
            }),
            updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:41:00Z")
                .expect("timestamp should parse")
                .with_timezone(&Utc),
            metadata: Some(ConnectorMetadata {
                connector_id: "cloud-primary".to_string(),
                source_kind: "cloud".to_string(),
                upstream_id: "checkpoint/sync-session".to_string(),
                revision: Some("r2".to_string()),
                observed_at_utc: DateTime::parse_from_rfc3339("2026-03-05T06:41:00Z")
                    .expect("timestamp should parse")
                    .with_timezone(&Utc),
                extra: Some(json!({ "endpoint": "cloud" })),
            }),
        })
        .await
        .expect("checkpoint update should succeed");

    let queries = client.queries().await;
    assert!(queries[0].contains("FROM sync_checkpoint"));
    assert!(queries[1].contains("UPSERT sync_checkpoint:"));
}

#[tokio::test(flavor = "current_thread")]
async fn batch_rekey_scopes_dry_run_reports_scope_counts() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    client
        .queue_response(vec![json!({
            "TenantId": "acme",
            "SessionId": "tenant:acme::session:source-session"
        })])
        .await;
    client.queue_response(vec![json!({ "Count": 3 })]).await;
    client.queue_response(vec![json!({ "Count": 2 })]).await;
    client.queue_response(vec![json!({ "Count": 0 })]).await;
    client.queue_response(vec![json!({ "Count": 0 })]).await;

    let result = store
        .batch_rekey_scopes_async(
            vec!["abc123".to_string()],
            "acme",
            "tenant:acme::session:target-session",
            true,
            false,
        )
        .await
        .expect("dry-run rekey should succeed");

    assert!(result.dry_run);
    assert_eq!(result.requested_node_ids, 1);
    assert_eq!(result.resolved_node_ids, 1);
    assert!(result.missing_node_ids.is_empty());
    assert_eq!(result.scopes.len(), 1);
    assert_eq!(result.scopes[0].temporal_nodes, 3);
    assert_eq!(result.scopes[0].calibrations, 2);
    assert!(!result.scopes[0].applied);

    let queries = client.queries().await;
    assert!(
        queries
            .iter()
            .any(|query| query.contains("WHERE id = type::record('temporal_node', $node_id)"))
    );
    assert!(
        !queries
            .iter()
            .any(|query| query.contains("BEGIN TRANSACTION"))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn batch_rekey_scopes_apply_updates_both_tables() {
    let client = Arc::new(MockSurrealDbClient::default());
    let store = SurrealDbNodeStore::new(client.clone());

    client
        .queue_response(vec![json!({
            "TenantId": "acme",
            "SessionId": "tenant:acme::session:source-session"
        })])
        .await;
    client.queue_response(vec![json!({ "Count": 4 })]).await;
    client.queue_response(vec![json!({ "Count": 1 })]).await;
    client.queue_response(vec![json!({ "Count": 0 })]).await;
    client.queue_response(vec![json!({ "Count": 0 })]).await;

    let result = store
        .batch_rekey_scopes_async(
            vec!["temporal_node:abc123".to_string()],
            "acme",
            "tenant:acme::session:target-session",
            false,
            false,
        )
        .await
        .expect("apply rekey should succeed");

    assert!(!result.dry_run);
    assert_eq!(result.temporal_nodes_updated, 4);
    assert_eq!(result.calibrations_updated, 1);
    assert_eq!(result.scopes.len(), 1);
    assert!(result.scopes[0].applied);
    assert!(!result.scopes[0].conflict);

    let queries = client.queries().await;
    assert!(
        queries
            .iter()
            .any(|query| query.contains("BEGIN TRANSACTION"))
    );

    let params = client.parameters().await;
    assert!(params.iter().any(|param| {
        param.get("target_tenant_id") == Some(&json!("acme"))
            && param.get("target_session_id") == Some(&json!("tenant:acme::session:target-session"))
    }));
}
