use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;

use crate::domain::contracts::{NodeStore, SyncChangeSource, SyncCoordinatorPolicy};
use crate::domain::models::{
    ConnectorMetadata, NodeUpsertStatus, SttpNode, SyncCheckpoint, SyncCursor, SyncPullRequest,
    SyncPullResult,
};

pub struct SyncCoordinatorService {
    store: Arc<dyn NodeStore>,
    source: Arc<dyn SyncChangeSource>,
    policy: Option<Arc<dyn SyncCoordinatorPolicy>>,
}

impl SyncCoordinatorService {
    pub fn new(store: Arc<dyn NodeStore>, source: Arc<dyn SyncChangeSource>) -> Self {
        Self {
            store,
            source,
            policy: None,
        }
    }

    pub fn with_policy(
        store: Arc<dyn NodeStore>,
        source: Arc<dyn SyncChangeSource>,
        policy: Arc<dyn SyncCoordinatorPolicy>,
    ) -> Self {
        Self {
            store,
            source,
            policy: Some(policy),
        }
    }

    pub async fn pull_async(&self, request: SyncPullRequest) -> Result<SyncPullResult> {
        let page_size = request.page_size.clamp(1, 500);
        let max_batches = request.max_batches.unwrap_or(usize::MAX).max(1);
        let mut checkpoint = self
            .store
            .get_checkpoint_async(&request.session_id, &request.connector_id)
            .await?;
        let mut cursor = checkpoint.as_ref().and_then(|value| value.cursor.clone());
        let mut result = SyncPullResult::default();

        while result.batches < max_batches {
            let page = self
                .source
                .read_changes_async(
                    &request.session_id,
                    &request.connector_id,
                    cursor.clone(),
                    page_size,
                )
                .await?;

            if page.nodes.is_empty() {
                result.has_more = page.has_more;
                break;
            }

            result.batches += 1;
            result.fetched += page.nodes.len();

            let mut last_applied_node: Option<SttpNode> = None;

            for node in page.nodes {
                if !self.should_accept_node(&node) {
                    result.filtered += 1;
                    continue;
                }

                let upsert = self.store.upsert_node_async(node.clone()).await?;
                last_applied_node = Some(node);

                match upsert.status {
                    NodeUpsertStatus::Created => result.created += 1,
                    NodeUpsertStatus::Updated => result.updated += 1,
                    NodeUpsertStatus::Duplicate => result.duplicate += 1,
                    NodeUpsertStatus::Skipped => result.skipped += 1,
                }
            }

            if let Some(next_cursor) = page.next_cursor {
                cursor = Some(next_cursor.clone());
                let next_checkpoint = SyncCheckpoint {
                    session_id: request.session_id.clone(),
                    connector_id: request.connector_id.clone(),
                    cursor: Some(next_cursor.clone()),
                    updated_at: Utc::now(),
                    metadata: self.build_checkpoint_metadata(
                        &request.session_id,
                        &request.connector_id,
                        checkpoint.as_ref(),
                        last_applied_node.as_ref(),
                        Some(&next_cursor),
                    ),
                };

                self.store
                    .put_checkpoint_async(next_checkpoint.clone())
                    .await?;
                checkpoint = Some(next_checkpoint);
            }

            result.has_more = page.has_more;
            if !result.has_more {
                break;
            }
        }

        result.last_cursor = cursor;
        result.checkpoint = checkpoint;
        Ok(result)
    }

    fn should_accept_node(&self, node: &SttpNode) -> bool {
        self.policy
            .as_ref()
            .map(|policy| policy.should_accept_node(node))
            .unwrap_or(true)
    }

    fn build_checkpoint_metadata(
        &self,
        session_id: &str,
        connector_id: &str,
        previous: Option<&SyncCheckpoint>,
        last_applied_node: Option<&SttpNode>,
        next_cursor: Option<&SyncCursor>,
    ) -> Option<ConnectorMetadata> {
        self.policy
            .as_ref()
            .and_then(|policy| {
                policy.checkpoint_metadata(
                    session_id,
                    connector_id,
                    previous,
                    last_applied_node,
                    next_cursor,
                )
            })
            .or_else(|| previous.and_then(|checkpoint| checkpoint.metadata.clone()))
    }
}
