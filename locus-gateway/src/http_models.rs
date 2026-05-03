use axum::Json;
use axum::http::HeaderValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalibrateSessionHttpRequest {
    pub(crate) session_id: String,
    pub(crate) tenant_id: Option<String>,
    pub(crate) stability: f32,
    pub(crate) friction: f32,
    pub(crate) logic: f32,
    pub(crate) autonomy: f32,
    pub(crate) trigger: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoreContextHttpRequest {
    pub(crate) node: String,
    pub(crate) session_id: String,
    pub(crate) tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScoreAvecHttpRequest {
    pub(crate) text: String,
    pub(crate) tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetContextHttpRequest {
    pub(crate) session_id: String,
    pub(crate) tenant_id: Option<String>,
    pub(crate) stability: f32,
    pub(crate) friction: f32,
    pub(crate) logic: f32,
    pub(crate) autonomy: f32,
    pub(crate) limit: Option<usize>,
    pub(crate) from_utc: Option<DateTime<Utc>>,
    pub(crate) to_utc: Option<DateTime<Utc>>,
    pub(crate) tiers: Option<Vec<String>>,
    pub(crate) query_text: Option<String>,
    pub(crate) query_embedding: Option<Vec<f32>>,
    pub(crate) alpha: Option<f32>,
    pub(crate) beta: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetEmbeddingContextHttpRequest {
    pub(crate) session_id: String,
    pub(crate) tenant_id: Option<String>,
    pub(crate) stability: f32,
    pub(crate) friction: f32,
    pub(crate) logic: f32,
    pub(crate) autonomy: f32,
    pub(crate) limit: Option<usize>,
    pub(crate) from_utc: Option<DateTime<Utc>>,
    pub(crate) to_utc: Option<DateTime<Utc>>,
    pub(crate) tiers: Option<Vec<String>>,
    pub(crate) rag_query_text: Option<String>,
    pub(crate) rag_embedding: Option<Vec<f32>>,
    pub(crate) avec_query_text: Option<String>,
    pub(crate) avec_embedding: Option<Vec<f32>>,
    pub(crate) rag_weight: Option<f32>,
    pub(crate) avec_weight: Option<f32>,
    pub(crate) alpha: Option<f32>,
    pub(crate) beta: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMonthlyRollupHttpRequest {
    pub(crate) session_id: String,
    pub(crate) tenant_id: Option<String>,
    pub(crate) start_date_utc: DateTime<Utc>,
    pub(crate) end_date_utc: DateTime<Utc>,
    pub(crate) source_session_id: Option<String>,
    pub(crate) parent_node_id: Option<String>,
    pub(crate) persist: Option<bool>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchRekeyHttpRequest {
    pub(crate) node_ids: Vec<String>,
    pub(crate) target_session_id: String,
    pub(crate) target_tenant_id: Option<String>,
    pub(crate) dry_run: Option<bool>,
    pub(crate) allow_merge: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingMigrationFilterHttp {
    pub(crate) session_id: Option<String>,
    pub(crate) from_utc: Option<DateTime<Utc>>,
    pub(crate) to_utc: Option<DateTime<Utc>>,
    pub(crate) tiers: Option<Vec<String>>,
    pub(crate) has_embedding: Option<bool>,
    pub(crate) embedding_model: Option<String>,
    pub(crate) sync_keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EmbeddingMigrationModeHttp {
    MissingOnly,
    ReindexAll,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingMigrationPreviewHttpRequest {
    pub(crate) tenant_id: Option<String>,
    pub(crate) filter: Option<EmbeddingMigrationFilterHttp>,
    pub(crate) sample_limit: Option<usize>,
    pub(crate) max_nodes: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingMigrationRunHttpRequest {
    pub(crate) tenant_id: Option<String>,
    pub(crate) filter: Option<EmbeddingMigrationFilterHttp>,
    pub(crate) mode: Option<EmbeddingMigrationModeHttp>,
    pub(crate) dry_run: Option<bool>,
    pub(crate) batch_size: Option<usize>,
    pub(crate) max_nodes: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListNodesQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) session_id: Option<String>,
    pub(crate) tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetMoodsQuery {
    pub(crate) target_mood: Option<String>,
    pub(crate) blend: Option<f32>,
    pub(crate) current_stability: Option<f32>,
    pub(crate) current_friction: Option<f32>,
    pub(crate) current_logic: Option<f32>,
    pub(crate) current_autonomy: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GraphQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) session_id: Option<String>,
    pub(crate) tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameSessionHttpRequest {
    pub(crate) source_session_id: String,
    pub(crate) target_session_id: String,
    pub(crate) tenant_id: Option<String>,
    pub(crate) allow_merge: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AvecStateDto {
    pub(crate) stability: f32,
    pub(crate) friction: f32,
    pub(crate) logic: f32,
    pub(crate) autonomy: f32,
    pub(crate) psi: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SttpNodeDto {
    pub(crate) raw: String,
    pub(crate) session_id: String,
    pub(crate) tier: String,
    pub(crate) timestamp: DateTime<Utc>,
    pub(crate) compression_depth: i32,
    pub(crate) parent_node_id: Option<String>,
    pub(crate) user_avec: AvecStateDto,
    pub(crate) model_avec: AvecStateDto,
    pub(crate) compression_avec: Option<AvecStateDto>,
    pub(crate) rho: f32,
    pub(crate) kappa: f32,
    pub(crate) psi: f32,
    pub(crate) sync_key: String,
    pub(crate) synthetic_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PsiRangeDto {
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) average: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NumericRangeDto {
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) average: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfidenceBandSummaryDto {
    pub(crate) low: usize,
    pub(crate) medium: usize,
    pub(crate) high: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalibrationResultDto {
    pub(crate) previous_avec: AvecStateDto,
    pub(crate) delta: f32,
    pub(crate) drift_classification: String,
    pub(crate) trigger: String,
    pub(crate) trigger_history: Vec<String>,
    pub(crate) is_first_calibration: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoreResultDto {
    pub(crate) node_id: String,
    pub(crate) psi: f32,
    pub(crate) valid: bool,
    pub(crate) validation_error: Option<String>,
    pub(crate) duplicate_skipped: bool,
    pub(crate) upsert_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScoreAvecResultDto {
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) avec: AvecStateDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameSessionResultDto {
    pub(crate) source_session_id: String,
    pub(crate) target_session_id: String,
    pub(crate) moved_nodes: usize,
    pub(crate) moved_calibrations: usize,
    pub(crate) scopes_applied: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrieveResultDto {
    pub(crate) nodes: Vec<SttpNodeDto>,
    pub(crate) retrieved: usize,
    pub(crate) psi_range: PsiRangeDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListNodesResultDto {
    pub(crate) nodes: Vec<SttpNodeDto>,
    pub(crate) retrieved: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoodPresetDto {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) avec: AvecStateDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoodSwapPreviewDto {
    pub(crate) target_mood: String,
    pub(crate) blend: f32,
    pub(crate) current: AvecStateDto,
    pub(crate) target: AvecStateDto,
    pub(crate) blended: AvecStateDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoodCatalogResultDto {
    pub(crate) presets: Vec<MoodPresetDto>,
    pub(crate) apply_guide: String,
    pub(crate) swap_preview: Option<MoodSwapPreviewDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MonthlyRollupResultDto {
    pub(crate) success: bool,
    pub(crate) node_id: String,
    pub(crate) raw_node: String,
    pub(crate) error: Option<String>,
    pub(crate) source_nodes: usize,
    pub(crate) parent_reference: Option<String>,
    pub(crate) user_average: AvecStateDto,
    pub(crate) model_average: AvecStateDto,
    pub(crate) compression_average: AvecStateDto,
    pub(crate) rho_range: NumericRangeDto,
    pub(crate) kappa_range: NumericRangeDto,
    pub(crate) psi_range: NumericRangeDto,
    pub(crate) rho_bands: ConfidenceBandSummaryDto,
    pub(crate) kappa_bands: ConfidenceBandSummaryDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScopeRekeyResultDto {
    pub(crate) source_tenant_id: String,
    pub(crate) source_session_id: String,
    pub(crate) target_tenant_id: String,
    pub(crate) target_session_id: String,
    pub(crate) temporal_nodes: usize,
    pub(crate) calibrations: usize,
    pub(crate) target_temporal_nodes: usize,
    pub(crate) target_calibrations: usize,
    pub(crate) applied: bool,
    pub(crate) conflict: bool,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchRekeyResultDto {
    pub(crate) dry_run: bool,
    pub(crate) requested_node_ids: usize,
    pub(crate) resolved_node_ids: usize,
    pub(crate) missing_node_ids: Vec<String>,
    pub(crate) scopes: Vec<ScopeRekeyResultDto>,
    pub(crate) temporal_nodes_updated: usize,
    pub(crate) calibrations_updated: usize,
    pub(crate) updated_scopes: usize,
    pub(crate) conflict_scopes: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingMigrationSampleDto {
    pub(crate) sync_key: String,
    pub(crate) session_id: String,
    pub(crate) tier: String,
    pub(crate) has_embedding: bool,
    pub(crate) embedding_model: Option<String>,
    pub(crate) embedding_dimensions: Option<usize>,
    pub(crate) embedded_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) context_summary: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingMigrationPreviewResultDto {
    pub(crate) total_candidates: usize,
    pub(crate) sample: Vec<EmbeddingMigrationSampleDto>,
    pub(crate) provider_available: bool,
    pub(crate) provider_model: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddingMigrationRunResultDto {
    pub(crate) scanned: usize,
    pub(crate) selected: usize,
    pub(crate) updated: usize,
    pub(crate) skipped: usize,
    pub(crate) failed: usize,
    pub(crate) duplicate: usize,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) completed_at: DateTime<Utc>,
    pub(crate) provider_model: Option<String>,
    pub(crate) dry_run: bool,
    pub(crate) mode: String,
    pub(crate) failure_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GraphResponse {
    pub(crate) sessions: Vec<serde_json::Value>,
    pub(crate) nodes: Vec<serde_json::Value>,
    pub(crate) edges: Vec<serde_json::Value>,
    pub(crate) retrieved: usize,
}

pub(crate) type ApiResult<T> = Result<Json<T>, (axum::http::StatusCode, Json<ErrorResponse>)>;

pub(crate) enum CorsAllowedOrigins {
    Any,
    Explicit(Vec<HeaderValue>),
}
