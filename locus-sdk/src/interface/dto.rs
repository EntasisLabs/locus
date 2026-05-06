use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use locus_core_rs::domain::models::{AvecState, PsiRange, SttpNode};

use crate::application::memory_composition::{
    CompositeInputItem, CompositeNodeFromTextOptions, CompositeNodeFromTextRequest,
    CompositeNodeFromTextResult, CompositeRole, CompositeRoleAvecOverrides,
    MemoryDailyRollupRequest, MemoryRecallWithExplainResult, MemoryTransformThenRecallRequest,
    MemoryTransformThenRecallResult,
};
use crate::domain::memory::{
    FallbackPolicy, MemoryAggregateRequest, MemoryAggregateResult, MemoryExplainRequest,
    MemoryExplainResult, MemoryFilter,
    MemoryFindRequest, MemoryFindResult, MemoryGroupBy, MemoryPage, MemoryRecallRequest,
    MemoryRecallResult, MemorySchemaResult, MemoryScope, MemoryScoring, MemorySort,
    MemoryTransformOperation,
    MemoryTransformRequest, MemoryTransformResult, MetricRange, NumericStats, RetrievalPath,
    StrictnessMode,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvecStateDto {
    pub stability: f32,
    pub friction: f32,
    pub logic: f32,
    pub autonomy: f32,
    pub psi: f32,
}

impl From<AvecStateDto> for AvecState {
    fn from(value: AvecStateDto) -> Self {
        Self {
            stability: value.stability,
            friction: value.friction,
            logic: value.logic,
            autonomy: value.autonomy,
        }
    }
}

impl From<AvecState> for AvecStateDto {
    fn from(value: AvecState) -> Self {
        Self {
            stability: value.stability,
            friction: value.friction,
            logic: value.logic,
            autonomy: value.autonomy,
            psi: value.psi(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PsiRangeDto {
    pub min: f32,
    pub max: f32,
    pub average: f32,
}

impl From<PsiRange> for PsiRangeDto {
    fn from(value: PsiRange) -> Self {
        Self {
            min: value.min,
            max: value.max,
            average: value.average,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScopeDto {
    pub tenant_id: Option<String>,
    pub session_ids: Option<Vec<String>>,
    pub tiers: Option<Vec<String>>,
    pub from_utc: Option<DateTime<Utc>>,
    pub to_utc: Option<DateTime<Utc>>,
}

impl From<MemoryScopeDto> for MemoryScope {
    fn from(value: MemoryScopeDto) -> Self {
        Self {
            tenant_id: value.tenant_id,
            session_ids: value.session_ids,
            tiers: value.tiers,
            from_utc: value.from_utc,
            to_utc: value.to_utc,
        }
    }
}

impl From<MemoryScope> for MemoryScopeDto {
    fn from(value: MemoryScope) -> Self {
        Self {
            tenant_id: value.tenant_id,
            session_ids: value.session_ids,
            tiers: value.tiers,
            from_utc: value.from_utc,
            to_utc: value.to_utc,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFilterDto {
    pub has_embedding: Option<bool>,
    pub embedding_model: Option<String>,
    pub psi: Option<MetricRange>,
    pub rho: Option<MetricRange>,
    pub kappa: Option<MetricRange>,
    pub text_contains: Option<String>,
}

impl From<MemoryFilterDto> for MemoryFilter {
    fn from(value: MemoryFilterDto) -> Self {
        Self {
            has_embedding: value.has_embedding,
            embedding_model: value.embedding_model,
            psi: value.psi,
            rho: value.rho,
            kappa: value.kappa,
            text_contains: value.text_contains,
        }
    }
}

impl From<MemoryFilter> for MemoryFilterDto {
    fn from(value: MemoryFilter) -> Self {
        Self {
            has_embedding: value.has_embedding,
            embedding_model: value.embedding_model,
            psi: value.psi,
            rho: value.rho,
            kappa: value.kappa,
            text_contains: value.text_contains,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPageDto {
    pub limit: usize,
    pub cursor: Option<String>,
}

impl From<MemoryPageDto> for MemoryPage {
    fn from(value: MemoryPageDto) -> Self {
        Self {
            limit: value.limit,
            cursor: value.cursor,
        }
    }
}

impl From<MemoryPage> for MemoryPageDto {
    fn from(value: MemoryPage) -> Self {
        Self {
            limit: value.limit,
            cursor: value.cursor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScoringDto {
    pub resonance_weight: f32,
    pub semantic_weight: f32,
    pub lexical_weight: f32,
    pub alpha: f32,
    pub beta: f32,
    pub fallback_policy: FallbackPolicy,
    pub strictness: StrictnessMode,
}

impl From<MemoryScoringDto> for MemoryScoring {
    fn from(value: MemoryScoringDto) -> Self {
        Self {
            resonance_weight: value.resonance_weight,
            semantic_weight: value.semantic_weight,
            lexical_weight: value.lexical_weight,
            alpha: value.alpha,
            beta: value.beta,
            fallback_policy: value.fallback_policy,
            strictness: value.strictness,
        }
    }
}

impl From<MemoryScoring> for MemoryScoringDto {
    fn from(value: MemoryScoring) -> Self {
        Self {
            resonance_weight: value.resonance_weight,
            semantic_weight: value.semantic_weight,
            lexical_weight: value.lexical_weight,
            alpha: value.alpha,
            beta: value.beta,
            fallback_policy: value.fallback_policy,
            strictness: value.strictness,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFindRequestDto {
    pub scope: MemoryScopeDto,
    pub filter: MemoryFilterDto,
    pub page: MemoryPageDto,
    pub sort: MemorySort,
}

impl From<MemoryFindRequestDto> for MemoryFindRequest {
    fn from(value: MemoryFindRequestDto) -> Self {
        Self {
            scope: value.scope.into(),
            filter: value.filter.into(),
            page: value.page.into(),
            sort: value.sort,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecallRequestDto {
    pub scope: MemoryScopeDto,
    pub filter: MemoryFilterDto,
    pub page: MemoryPageDto,
    pub scoring: MemoryScoringDto,
    pub current_avec: Option<AvecStateDto>,
    pub query_text: Option<String>,
    pub query_embedding: Option<Vec<f32>>,
}

impl From<MemoryRecallRequestDto> for MemoryRecallRequest {
    fn from(value: MemoryRecallRequestDto) -> Self {
        Self {
            scope: value.scope.into(),
            filter: value.filter.into(),
            page: value.page.into(),
            scoring: value.scoring.into(),
            current_avec: value.current_avec.map(Into::into),
            query_text: value.query_text,
            query_embedding: value.query_embedding,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNodeDto {
    pub raw: String,
    pub session_id: String,
    pub tier: String,
    pub timestamp: DateTime<Utc>,
    pub compression_depth: i32,
    pub parent_node_id: Option<String>,
    pub sync_key: String,
    pub context_summary: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimensions: Option<usize>,
    pub embedded_at: Option<DateTime<Utc>>,
    pub rho: f32,
    pub kappa: f32,
    pub psi: f32,
    pub user_avec: AvecStateDto,
    pub model_avec: AvecStateDto,
    pub compression_avec: Option<AvecStateDto>,
    pub updated_at: DateTime<Utc>,
}

impl From<SttpNode> for MemoryNodeDto {
    fn from(value: SttpNode) -> Self {
        Self {
            raw: value.raw,
            session_id: value.session_id,
            tier: value.tier,
            timestamp: value.timestamp,
            compression_depth: value.compression_depth,
            parent_node_id: value.parent_node_id,
            sync_key: value.sync_key,
            context_summary: value.context_summary,
            embedding_model: value.embedding_model,
            embedding_dimensions: value.embedding_dimensions,
            embedded_at: value.embedded_at,
            rho: value.rho,
            kappa: value.kappa,
            psi: value.psi,
            user_avec: value.user_avec.into(),
            model_avec: value.model_avec.into(),
            compression_avec: value.compression_avec.map(Into::into),
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFindResponseDto {
    pub nodes: Vec<MemoryNodeDto>,
    pub retrieved: usize,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl From<MemoryFindResult> for MemoryFindResponseDto {
    fn from(value: MemoryFindResult) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            retrieved: value.retrieved,
            has_more: value.has_more,
            next_cursor: value.next_cursor,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecallResponseDto {
    pub nodes: Vec<MemoryNodeDto>,
    pub retrieved: usize,
    pub psi_range: PsiRangeDto,
    pub retrieval_path: RetrievalPath,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl From<MemoryRecallResult> for MemoryRecallResponseDto {
    fn from(value: MemoryRecallResult) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            retrieved: value.retrieved,
            psi_range: value.psi_range.into(),
            retrieval_path: value.retrieval_path,
            has_more: value.has_more,
            next_cursor: value.next_cursor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumericStatsDto {
    pub min: f32,
    pub max: f32,
    pub average: f32,
}

impl From<NumericStats> for NumericStatsDto {
    fn from(value: NumericStats) -> Self {
        Self {
            min: value.min,
            max: value.max,
            average: value.average,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAggregateRequestDto {
    pub scope: MemoryScopeDto,
    pub filter: MemoryFilterDto,
    pub group_by: MemoryGroupBy,
    pub max_groups: usize,
    pub max_nodes: usize,
}

impl From<MemoryAggregateRequestDto> for MemoryAggregateRequest {
    fn from(value: MemoryAggregateRequestDto) -> Self {
        Self {
            scope: value.scope.into(),
            filter: value.filter.into(),
            group_by: value.group_by,
            max_groups: value.max_groups,
            max_nodes: value.max_nodes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAggregateGroupDto {
    pub key: String,
    pub node_count: usize,
    pub embedding_coverage: f32,
    pub avg_user_avec: AvecStateDto,
    pub avg_model_avec: AvecStateDto,
    pub avg_compression_avec: Option<AvecStateDto>,
    pub psi_stats: NumericStatsDto,
    pub rho_stats: NumericStatsDto,
    pub kappa_stats: NumericStatsDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAggregateResponseDto {
    pub groups: Vec<MemoryAggregateGroupDto>,
    pub total_groups: usize,
    pub scanned_nodes: usize,
}

impl From<MemoryAggregateResult> for MemoryAggregateResponseDto {
    fn from(value: MemoryAggregateResult) -> Self {
        Self {
            groups: value
                .groups
                .into_iter()
                .map(|group| MemoryAggregateGroupDto {
                    key: group.key,
                    node_count: group.node_count,
                    embedding_coverage: group.embedding_coverage,
                    avg_user_avec: group.avg_user_avec.into(),
                    avg_model_avec: group.avg_model_avec.into(),
                    avg_compression_avec: group.avg_compression_avec.map(Into::into),
                    psi_stats: group.psi_stats.into(),
                    rho_stats: group.rho_stats.into(),
                    kappa_stats: group.kappa_stats.into(),
                })
                .collect(),
            total_groups: value.total_groups,
            scanned_nodes: value.scanned_nodes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTransformRequestDto {
    pub scope: MemoryScopeDto,
    pub filter: MemoryFilterDto,
    pub operation: MemoryTransformOperation,
    pub dry_run: bool,
    pub batch_size: usize,
    pub max_nodes: usize,
    pub provider_id: Option<String>,
    pub model: Option<String>,
}

impl From<MemoryTransformRequestDto> for MemoryTransformRequest {
    fn from(value: MemoryTransformRequestDto) -> Self {
        Self {
            scope: value.scope.into(),
            filter: value.filter.into(),
            operation: value.operation,
            dry_run: value.dry_run,
            batch_size: value.batch_size,
            max_nodes: value.max_nodes,
            provider_id: value.provider_id,
            model: value.model,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTransformResponseDto {
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

impl From<MemoryTransformResult> for MemoryTransformResponseDto {
    fn from(value: MemoryTransformResult) -> Self {
        Self {
            scanned: value.scanned,
            selected: value.selected,
            updated: value.updated,
            skipped: value.skipped,
            failed: value.failed,
            duplicate: value.duplicate,
            started_at: value.started_at,
            completed_at: value.completed_at,
            failures: value.failures,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDailyRollupRequestDto {
    pub scope: MemoryScopeDto,
    pub filter: MemoryFilterDto,
    pub max_days: usize,
    pub max_nodes: usize,
}

impl From<MemoryDailyRollupRequestDto> for MemoryDailyRollupRequest {
    fn from(value: MemoryDailyRollupRequestDto) -> Self {
        Self {
            scope: value.scope.into(),
            filter: value.filter.into(),
            max_days: value.max_days,
            max_nodes: value.max_nodes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecallWithExplainResponseDto {
    pub recall: MemoryRecallResponseDto,
    pub explain: MemoryExplainResponseDto,
}

impl From<MemoryRecallWithExplainResult> for MemoryRecallWithExplainResponseDto {
    fn from(value: MemoryRecallWithExplainResult) -> Self {
        Self {
            recall: value.recall.into(),
            explain: value.explain.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTransformThenRecallRequestDto {
    pub transform: MemoryTransformRequestDto,
    pub recall: MemoryRecallRequestDto,
}

impl From<MemoryTransformThenRecallRequestDto> for MemoryTransformThenRecallRequest {
    fn from(value: MemoryTransformThenRecallRequestDto) -> Self {
        Self {
            transform: value.transform.into(),
            recall: value.recall.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTransformThenRecallResponseDto {
    pub transform: MemoryTransformResponseDto,
    pub recall: MemoryRecallResponseDto,
}

impl From<MemoryTransformThenRecallResult> for MemoryTransformThenRecallResponseDto {
    fn from(value: MemoryTransformThenRecallResult) -> Self {
        Self {
            transform: value.transform.into(),
            recall: value.recall.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeRoleDto {
    User,
    Model,
    Document,
    Conversation,
}

impl From<CompositeRoleDto> for CompositeRole {
    fn from(value: CompositeRoleDto) -> Self {
        match value {
            CompositeRoleDto::User => CompositeRole::User,
            CompositeRoleDto::Model => CompositeRole::Model,
            CompositeRoleDto::Document => CompositeRole::Document,
            CompositeRoleDto::Conversation => CompositeRole::Conversation,
        }
    }
}

impl From<CompositeRole> for CompositeRoleDto {
    fn from(value: CompositeRole) -> Self {
        match value {
            CompositeRole::User => CompositeRoleDto::User,
            CompositeRole::Model => CompositeRoleDto::Model,
            CompositeRole::Document => CompositeRoleDto::Document,
            CompositeRole::Conversation => CompositeRoleDto::Conversation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeInputItemDto {
    pub role: CompositeRoleDto,
    pub text: String,
    pub avec_override: Option<AvecStateDto>,
    #[serde(default)]
    pub context: Vec<CompositeInputItemDto>,
}

impl From<CompositeInputItemDto> for CompositeInputItem {
    fn from(value: CompositeInputItemDto) -> Self {
        Self {
            role: value.role.into(),
            text: value.text,
            avec_override: value.avec_override.map(Into::into),
            context: value.context.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CompositeInputItem> for CompositeInputItemDto {
    fn from(value: CompositeInputItem) -> Self {
        Self {
            role: value.role.into(),
            text: value.text,
            avec_override: value.avec_override.map(Into::into),
            context: value.context.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeRoleAvecOverridesDto {
    pub user: Option<AvecStateDto>,
    pub model: Option<AvecStateDto>,
    pub document: Option<AvecStateDto>,
    pub conversation: Option<AvecStateDto>,
}

impl From<CompositeRoleAvecOverridesDto> for CompositeRoleAvecOverrides {
    fn from(value: CompositeRoleAvecOverridesDto) -> Self {
        Self {
            user: value.user.map(Into::into),
            model: value.model.map(Into::into),
            document: value.document.map(Into::into),
            conversation: value.conversation.map(Into::into),
        }
    }
}

impl From<CompositeRoleAvecOverrides> for CompositeRoleAvecOverridesDto {
    fn from(value: CompositeRoleAvecOverrides) -> Self {
        Self {
            user: value.user.map(Into::into),
            model: value.model.map(Into::into),
            document: value.document.map(Into::into),
            conversation: value.conversation.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeNodeFromTextOptionsDto {
    pub role_avec: CompositeRoleAvecOverridesDto,
    pub global_avec: Option<AvecStateDto>,
    pub allow_llm_avec_fallback: bool,
    pub max_recursion_depth: usize,
}

impl From<CompositeNodeFromTextOptionsDto> for CompositeNodeFromTextOptions {
    fn from(value: CompositeNodeFromTextOptionsDto) -> Self {
        Self {
            role_avec: value.role_avec.into(),
            global_avec: value.global_avec.map(Into::into),
            allow_llm_avec_fallback: value.allow_llm_avec_fallback,
            max_recursion_depth: value.max_recursion_depth,
        }
    }
}

impl From<CompositeNodeFromTextOptions> for CompositeNodeFromTextOptionsDto {
    fn from(value: CompositeNodeFromTextOptions) -> Self {
        Self {
            role_avec: value.role_avec.into(),
            global_avec: value.global_avec.map(Into::into),
            allow_llm_avec_fallback: value.allow_llm_avec_fallback,
            max_recursion_depth: value.max_recursion_depth,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeNodeFromTextRequestDto {
    pub items: Vec<CompositeInputItemDto>,
    pub options: CompositeNodeFromTextOptionsDto,
}

impl From<CompositeNodeFromTextRequestDto> for CompositeNodeFromTextRequest {
    fn from(value: CompositeNodeFromTextRequestDto) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            options: value.options.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositeNodeFromTextResponseDto {
    pub content: Value,
    pub resolved_avec_count: usize,
    pub unresolved_avec_count: usize,
    pub requires_llm_avec: bool,
}

impl From<CompositeNodeFromTextResult> for CompositeNodeFromTextResponseDto {
    fn from(value: CompositeNodeFromTextResult) -> Self {
        Self {
            content: value.content,
            resolved_avec_count: value.resolved_avec_count,
            unresolved_avec_count: value.unresolved_avec_count,
            requires_llm_avec: value.requires_llm_avec,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryExplainRequestDto {
    pub recall: MemoryRecallRequestDto,
}

impl From<MemoryExplainRequestDto> for MemoryExplainRequest {
    fn from(value: MemoryExplainRequestDto) -> Self {
        Self {
            recall: value.recall.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryExplainStageDto {
    pub stage: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryExplainResponseDto {
    pub retrieval_path: RetrievalPath,
    pub fallback_triggered: bool,
    pub fallback_reason: Option<String>,
    pub stages: Vec<MemoryExplainStageDto>,
    pub scoring: MemoryScoringDto,
}

impl From<MemoryExplainResult> for MemoryExplainResponseDto {
    fn from(value: MemoryExplainResult) -> Self {
        Self {
            retrieval_path: value.retrieval_path,
            fallback_triggered: value.fallback_triggered,
            fallback_reason: value.fallback_reason,
            stages: value
                .stages
                .into_iter()
                .map(|stage| MemoryExplainStageDto {
                    stage: stage.stage,
                    count: stage.count,
                })
                .collect(),
            scoring: value.scoring.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySchemaResponseDto {
    pub schema_version: String,
    pub sort_fields: Vec<String>,
    pub filter_fields: Vec<String>,
    pub group_by_fields: Vec<String>,
    pub fallback_policies: Vec<String>,
    pub strictness_modes: Vec<String>,
    pub transform_operations: Vec<String>,
}

impl From<MemorySchemaResult> for MemorySchemaResponseDto {
    fn from(value: MemorySchemaResult) -> Self {
        Self {
            schema_version: value.schema_version,
            sort_fields: value.sort_fields,
            filter_fields: value.filter_fields,
            group_by_fields: value.group_by_fields,
            fallback_policies: value.fallback_policies,
            strictness_modes: value.strictness_modes,
            transform_operations: value.transform_operations,
        }
    }
}
