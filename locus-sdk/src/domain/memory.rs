use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use locus_core_rs::domain::models::{AvecState, PsiRange, SttpNode};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackPolicy {
    Never,
    OnEmpty,
    Always,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self::OnEmpty
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrictnessMode {
    Precision,
    Balanced,
    Recall,
}

impl Default for StrictnessMode {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySortField {
    Timestamp,
    UpdatedAt,
    Psi,
    Rho,
    Kappa,
}

impl Default for MemorySortField {
    fn default() -> Self {
        Self::Timestamp
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Desc
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalPath {
    ResonanceOnly,
    SemanticOnly,
    Hybrid,
    LexicalFallback,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScope {
    pub tenant_id: Option<String>,
    pub session_ids: Option<Vec<String>>,
    pub tiers: Option<Vec<String>>,
    pub from_utc: Option<DateTime<Utc>>,
    pub to_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricRange {
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl MetricRange {
    pub fn contains(&self, value: f32) -> bool {
        if let Some(min) = self.min {
            if value < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFilter {
    pub has_embedding: Option<bool>,
    pub embedding_model: Option<String>,
    pub psi: Option<MetricRange>,
    pub rho: Option<MetricRange>,
    pub kappa: Option<MetricRange>,
    pub text_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPage {
    pub limit: usize,
    pub cursor: Option<String>,
}

impl Default for MemoryPage {
    fn default() -> Self {
        Self {
            limit: 50,
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySort {
    pub field: MemorySortField,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScoring {
    pub resonance_weight: f32,
    pub semantic_weight: f32,
    pub lexical_weight: f32,
    pub alpha: f32,
    pub beta: f32,
    pub fallback_policy: FallbackPolicy,
    pub strictness: StrictnessMode,
}

impl Default for MemoryScoring {
    fn default() -> Self {
        Self {
            resonance_weight: 1.0,
            semantic_weight: 0.0,
            lexical_weight: 0.0,
            alpha: 0.7,
            beta: 0.3,
            fallback_policy: FallbackPolicy::OnEmpty,
            strictness: StrictnessMode::Balanced,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFindRequest {
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub page: MemoryPage,
    pub sort: MemorySort,
}

#[derive(Debug, Clone)]
pub struct MemoryFindResult {
    pub nodes: Vec<SttpNode>,
    pub retrieved: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryRecallRequest {
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub page: MemoryPage,
    pub scoring: MemoryScoring,
    pub current_avec: Option<AvecState>,
    pub query_text: Option<String>,
    pub query_embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct MemoryRecallResult {
    pub nodes: Vec<SttpNode>,
    pub retrieved: usize,
    pub psi_range: PsiRange,
    pub retrieval_path: RetrievalPath,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MemoryExplainRequest {
    pub recall: MemoryRecallRequest,
}

#[derive(Debug, Clone)]
pub struct MemoryExplainStage {
    pub stage: String,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryExplainResult {
    pub retrieval_path: RetrievalPath,
    pub fallback_triggered: bool,
    pub fallback_reason: Option<String>,
    pub stages: Vec<MemoryExplainStage>,
    pub scoring: MemoryScoring,
}

#[derive(Debug, Clone, Default)]
pub struct MemorySchemaResult {
    pub schema_version: String,
    pub sort_fields: Vec<String>,
    pub filter_fields: Vec<String>,
    pub group_by_fields: Vec<String>,
    pub fallback_policies: Vec<String>,
    pub strictness_modes: Vec<String>,
    pub transform_operations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryGroupBy {
    SessionId,
    Tier,
    EmbeddingModel,
    DateDay,
}

impl Default for MemoryGroupBy {
    fn default() -> Self {
        Self::SessionId
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NumericStats {
    pub min: f32,
    pub max: f32,
    pub average: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAggregateRequest {
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub group_by: MemoryGroupBy,
    pub max_groups: usize,
    pub max_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryAggregateGroup {
    pub key: String,
    pub node_count: usize,
    pub embedding_coverage: f32,
    pub avg_user_avec: AvecState,
    pub avg_model_avec: AvecState,
    pub avg_compression_avec: Option<AvecState>,
    pub psi_stats: NumericStats,
    pub rho_stats: NumericStats,
    pub kappa_stats: NumericStats,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryAggregateResult {
    pub groups: Vec<MemoryAggregateGroup>,
    pub total_groups: usize,
    pub scanned_nodes: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTransformOperation {
    EmbedBackfill,
    ReindexEmbeddings,
}

impl Default for MemoryTransformOperation {
    fn default() -> Self {
        Self::EmbedBackfill
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTransformRequest {
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub operation: MemoryTransformOperation,
    pub dry_run: bool,
    pub batch_size: usize,
    pub max_nodes: usize,
    pub provider_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryTransformResult {
    pub scanned: usize,
    pub selected: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub duplicate: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub failures: Vec<String>,
}

pub fn clamp_limit(limit: usize) -> usize {
    limit.clamp(1, 200)
}

pub fn clamp_groups(limit: usize) -> usize {
    limit.clamp(1, 5000)
}

pub fn clamp_nodes(limit: usize) -> usize {
    limit.clamp(1, 50000)
}

pub fn clamp_batch_size(limit: usize) -> usize {
    limit.clamp(1, 500)
}
