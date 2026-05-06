use std::sync::Arc;

use anyhow::Result;

use crate::domain::contracts::NodeStore;
use crate::domain::models::BatchRekeyResult;

pub struct RekeyScopeService {
    store: Arc<dyn NodeStore>,
}

impl RekeyScopeService {
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self { store }
    }

    pub async fn rekey_async(
        &self,
        node_ids: Vec<String>,
        target_tenant_id: &str,
        target_session_id: &str,
        dry_run: bool,
        allow_merge: bool,
    ) -> Result<BatchRekeyResult> {
        self.store
            .batch_rekey_scopes_async(
                node_ids,
                target_tenant_id,
                target_session_id,
                dry_run,
                allow_merge,
            )
            .await
    }
}
