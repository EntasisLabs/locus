use serde::{Deserialize, Serialize};

use crate::domain::memory::{MemoryFilter, MemoryScope};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEvictMode {
    #[default]
    BySyncKeys,
    ByNodeIds,
    ByFilter,
    PurgeSession,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEvictRequest {
    pub mode: MemoryEvictMode,
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub sync_keys: Option<Vec<String>>,
    pub node_ids: Option<Vec<String>>,
    pub dry_run: bool,
    pub force: bool,
    pub max_nodes: usize,
    pub include_calibration: bool,
    pub include_checkpoints: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEvictRecord {
    pub node_id: String,
    pub sync_key: String,
    pub status: String,
    pub reason: Option<String>,
    pub inbound_references: Option<InboundReferencesPreview>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundReferencesPreview {
    pub child_parent_links: Vec<String>,
    pub incoming_semantic_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEvictResult {
    pub dry_run: bool,
    pub deleted: usize,
    pub blocked: usize,
    pub not_found: usize,
    pub skipped: usize,
    pub would_delete: Vec<String>,
    pub calibrations_deleted: usize,
    pub checkpoints_deleted: usize,
    pub records: Vec<MemoryEvictRecord>,
}
