use std::collections::VecDeque;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use locus_core::application::services::SyncCoordinatorService;
use locus_core::domain::contracts::{NodeStore, SyncChangeSource, SyncCoordinatorPolicy};
use locus_core::domain::models::{
    AvecState, ChangeQueryResult, SttpNode, SyncCursor, SyncPullRequest,
};
use locus_core::storage::InMemoryNodeStore;

struct StubChangeSource {
    pages: Mutex<VecDeque<ChangeQueryResult>>,
}

impl StubChangeSource {
    fn new(pages: Vec<ChangeQueryResult>) -> Self {
        Self {
            pages: Mutex::new(VecDeque::from(pages)),
        }
    }
}

#[async_trait]
impl SyncChangeSource for StubChangeSource {
    async fn read_changes_async(
        &self,
        _session_id: &str,
        _connector_id: &str,
        _cursor: Option<SyncCursor>,
        _limit: usize,
    ) -> Result<ChangeQueryResult> {
        Ok(self.pages.lock().await.pop_front().unwrap_or_default())
    }
}

struct RejectSkipPolicy;

impl SyncCoordinatorPolicy for RejectSkipPolicy {
    fn should_accept_node(&self, node: &SttpNode) -> bool {
        node.raw != "skip"
    }
}

fn build_test_node(session_id: &str, raw: &str, sync_key: &str, updated_at: &str) -> SttpNode {
    SttpNode {
        raw: raw.to_string(),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp: DateTime::parse_from_rfc3339("2026-03-05T06:30:00Z")
            .expect("timestamp should parse")
            .with_timezone(&Utc),
        compression_depth: 1,
        parent_node_id: None,
        sync_key: sync_key.to_string(),
        updated_at: DateTime::parse_from_rfc3339(updated_at)
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
async fn pull_does_not_resurface_remote_rows_as_local_changes() {
    let store = Arc::new(InMemoryNodeStore::new());
    let source = Arc::new(StubChangeSource::new(vec![ChangeQueryResult {
        nodes: vec![build_test_node(
            "sync-session",
            "remote",
            "sync-a",
            "2026-03-05T06:41:00Z",
        )],
        next_cursor: Some(SyncCursor {
            updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:41:00Z")
                .expect("timestamp should parse")
                .with_timezone(&Utc),
            sync_key: "sync-a".to_string(),
        }),
        has_more: false,
    }]));
    let coordinator = SyncCoordinatorService::new(store.clone(), source);

    let result = coordinator
        .pull_async(SyncPullRequest {
            session_id: "sync-session".to_string(),
            connector_id: "cloud-primary".to_string(),
            page_size: 50,
            max_batches: Some(1),
        })
        .await
        .expect("pull should succeed");

    let changes = store
        .query_changes_since_async(
            "sync-session",
            result
                .checkpoint
                .as_ref()
                .and_then(|checkpoint| checkpoint.cursor.clone()),
            50,
        )
        .await
        .expect("change query should succeed");

    assert!(changes.nodes.is_empty());
    assert!(!changes.has_more);
}

#[tokio::test(flavor = "current_thread")]
async fn coordinator_pages_changes_and_advances_checkpoint_without_owning_policy() {
    let store = Arc::new(InMemoryNodeStore::new());
    let source = Arc::new(StubChangeSource::new(vec![ChangeQueryResult {
        nodes: vec![
            build_test_node("sync-session", "apply", "sync-a", "2026-03-05T06:31:00Z"),
            build_test_node("sync-session", "skip", "sync-b", "2026-03-05T06:32:00Z"),
        ],
        next_cursor: Some(SyncCursor {
            updated_at: DateTime::parse_from_rfc3339("2026-03-05T06:32:00Z")
                .expect("timestamp should parse")
                .with_timezone(&Utc),
            sync_key: "sync-b".to_string(),
        }),
        has_more: false,
    }]));
    let coordinator =
        SyncCoordinatorService::with_policy(store.clone(), source, Arc::new(RejectSkipPolicy));

    let result = coordinator
        .pull_async(SyncPullRequest {
            session_id: "sync-session".to_string(),
            connector_id: "cloud-primary".to_string(),
            page_size: 50,
            max_batches: Some(1),
        })
        .await
        .expect("pull should succeed");

    assert_eq!(result.fetched, 2);
    assert_eq!(result.created, 1);
    assert_eq!(result.filtered, 1);
    assert!(!result.has_more);
    assert_eq!(
        result
            .checkpoint
            .as_ref()
            .and_then(|checkpoint| checkpoint.cursor.as_ref())
            .map(|cursor| cursor.sync_key.as_str()),
        Some("sync-b")
    );

    let stored = store
        .query_nodes_async(locus_core::domain::models::NodeQuery {
            limit: 10,
            session_id: Some("sync-session".to_string()),
            from_utc: None,
            to_utc: None,
            tiers: None,
        })
        .await
        .expect("query should succeed");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].raw, "apply");
}
