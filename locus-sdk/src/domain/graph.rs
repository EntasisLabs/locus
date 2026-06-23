use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::memory::{MemoryFilter, MemoryScope};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGraphRequest {
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub include_lineage: bool,
    pub include_semantic: bool,
    pub include_session_topology: bool,
    pub rel: Option<String>,
    pub target_prefix: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryGraphResult {
    pub sessions: Vec<Value>,
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
    pub retrieved: usize,
}
