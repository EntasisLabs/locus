//! `locus-mcp` binary.
//!
//! Exposes STTP memory operations over the Model Context Protocol (MCP)
//! for assistant and agent runtimes.

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use locus_core::domain::contracts::EmbeddingProvider;
use locus_core::{
    CalibrationService, EmbeddingMigrationFilter, EmbeddingMigrationMode,
    EmbeddingMigrationPreviewRequest, EmbeddingMigrationRunRequest, EmbeddingMigrationService,
    InMemoryNodeStore, MonthlyRollupRequest, MonthlyRollupService, MoodCatalogService, NodeStore,
    NodeStoreInitializer, NodeValidator, StoreContextService, SurrealDbClient, SurrealDbNodeStore,
    SurrealDbRuntimeOptions, SurrealDbSettings, TreeSitterValidator,
};
use locus_sdk::application::memory_recall::MemoryRecallService;
use locus_sdk::application::memory_find::MemoryFindService;
use locus_sdk::domain::memory::{MemoryFindRequest, MemoryPage, MemoryRecallRequest, MemoryScope, MemoryScoring};
#[cfg(feature = "local-embedding")]
use locus_sdk::infrastructure::embeddings::LocalEmbeddingProvider;
use locus_sdk::infrastructure::embeddings::OllamaEmbeddingProvider;
use surrealdb::engine::any::{Any, connect};
use surrealdb::opt::auth::Root;
use tracing::{error, info};

#[derive(Debug, Clone)]
enum EmbeddingsProviderKind {
    Ollama,
    #[cfg(feature = "local-embedding")]
    Local,
}

impl EmbeddingsProviderKind {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ollama" => Some(Self::Ollama),
            #[cfg(feature = "local-embedding")]
            "local" | "local-embedding" | "candle" => Some(Self::Local),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct SttpMcpServer {
    node_store: Arc<dyn NodeStore>,
    calibration: Arc<CalibrationService>,
    store_context: Arc<StoreContextService>,
    embedding_migration: Arc<EmbeddingMigrationService>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    moods: Arc<MoodCatalogService>,
    monthly_rollup: Arc<MonthlyRollupService>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl SttpMcpServer {
    fn new(
        node_store: Arc<dyn NodeStore>,
        calibration: Arc<CalibrationService>,
        store_context: Arc<StoreContextService>,
        embedding_migration: Arc<EmbeddingMigrationService>,
        embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
        moods: Arc<MoodCatalogService>,
        monthly_rollup: Arc<MonthlyRollupService>,
    ) -> Self {
        Self {
            node_store,
            calibration,
            store_context,
            embedding_migration,
            embedding_provider,
            moods,
            monthly_rollup,
            tool_router: Self::tool_router(),
        }
    }

    async fn embed_context_keywords(&self, keywords: &[String]) -> Option<Vec<f32>> {
        let provider = self.embedding_provider.as_ref()?;
        let prompt = keywords.join(" ");
        let prompt = prompt.trim();

        if prompt.is_empty() {
            return None;
        }

        provider
            .embed_async(prompt)
            .await
            .ok()
            .filter(|vector| !vector.is_empty())
    }
}

#[tool_router]
impl SttpMcpServer {
    #[tool(
        name = "calibrate_session",
        description = "Call this at session start and after heavy reasoning work to measure current AVEC drift. Use it to compare your current cognitive state against prior calibration for the same session before storing or retrieving memory."
    )]
    async fn calibrate_session(
        &self,
        Parameters(request): Parameters<CalibrateSessionRequest>,
    ) -> String {
        match self
            .calibration
            .calibrate_async(
                &request.session_id,
                request.stability,
                request.friction,
                request.logic,
                request.autonomy,
                &request.trigger,
            )
            .await
        {
            Ok(result) => to_json_string(json!({
                "previous_avec": avec_to_json(result.previous_avec),
                "delta": result.delta,
                "drift_classification": format!("{:?}", result.drift_classification),
                "trigger": result.trigger,
                "trigger_history": result.trigger_history,
                "is_first_calibration": result.is_first_calibration,
            })),
            Err(err) => {
                error!(error = %err, "calibrate_session failed");
                tool_error("CalibrationFailure", &err.to_string())
            }
        }
    }

    #[tool(
        name = "store_context",
        description = "Call this when context should be preserved to memory. Store a complete valid STTP node so future retrieval can rehydrate prior reasoning state, decisions, and confidence signals."
    )]
    async fn store_context(&self, Parameters(request): Parameters<StoreContextRequest>) -> String {
        let result = self
            .store_context
            .store_async(&request.node, &request.session_id)
            .await;

        to_json_string(json!({
            "node_id": result.node_id,
            "psi": result.psi,
            "valid": result.valid,
            "validation_error": result.validation_error,
        }))
    }

    #[tool(
        name = "get_context",
        description = "Primary memory retrieval tool (resonance search, not inventory listing). Returns top resonant memory nodes for the provided AVEC state. Optional context_keywords enables server-side semantic retrieval (with internal embedding generation); keyword fallback is only used when semantic retrieval returns no nodes (or embeddings are unavailable). If session_id is omitted, retrieval is global across sessions. Use list_nodes for inventory and debugging."
    )]
    async fn get_context(&self, Parameters(request): Parameters<GetContextRequest>) -> String {
        let from_utc = match parse_utc_optional(request.from_utc.as_deref(), "from_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };
        let to_utc = match parse_utc_optional(request.to_utc.as_deref(), "to_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };

        let tiers = request
            .tiers
            .as_ref()
            .map(|values| normalize_tiers(values.as_slice()));

        let limit = match validate_limit(request.limit, "limit") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };
        let context_keywords = normalize_context_keywords(request.context_keywords.as_deref());

        if let Some(alpha) = request.alpha {
            if !(0.0..=1.0).contains(&alpha) {
                return tool_error("InvalidArgument", "alpha must be between 0.0 and 1.0");
            }
        }
        if let Some(beta) = request.beta {
            if !(0.0..=1.0).contains(&beta) {
                return tool_error("InvalidArgument", "beta must be between 0.0 and 1.0");
            }
        }

        let alpha = request.alpha.unwrap_or(0.7);
        let beta = request.beta.unwrap_or(0.3);
        let query_text = if context_keywords.is_empty() {
            None
        } else {
            Some(context_keywords.join(" "))
        };
        let query_embedding = if context_keywords.is_empty() {
            None
        } else {
            self.embed_context_keywords(&context_keywords).await
        };

        let recall_service = MemoryRecallService::new(self.node_store.clone());
        let recall_result = match recall_service
            .execute(&MemoryRecallRequest {
                scope: MemoryScope {
                    tenant_id: None,
                    session_ids: request.session_id.map(|session| vec![session]),
                    tiers,
                    from_utc,
                    to_utc,
                },
                page: MemoryPage {
                    limit,
                    cursor: None,
                },
                scoring: MemoryScoring {
                    alpha,
                    beta,
                    ..Default::default()
                },
                current_avec: Some(locus_core::AvecState {
                    stability: request.stability,
                    friction: request.friction,
                    logic: request.logic,
                    autonomy: request.autonomy,
                }),
                query_text,
                query_embedding,
                ..Default::default()
            })
            .await
        {
            Ok(result) => result,
            Err(err) => {
                error!(error = %err, "get_context failed");
                return tool_error("GetContextFailure", &err.to_string());
            }
        };

        to_json_string(json!({
            "retrieved": recall_result.retrieved,
            "psi_range": {
                "min": recall_result.psi_range.min,
                "max": recall_result.psi_range.max,
                "average": recall_result.psi_range.average,
            },
            "nodes": recall_result
                .nodes
                .iter()
                .map(sttp_node_to_json)
                .collect::<Vec<_>>(),
        }))
    }

    #[tool(
        name = "list_nodes",
        description = "Memory inventory tool. Lists stored nodes newest-first (global when session_id is omitted). Optional context_keywords performs fuzzy filtering against context_summary for fast discovery. Unlike get_context, list_nodes does not perform AVEC resonance ranking."
    )]
    async fn list_nodes(&self, Parameters(request): Parameters<ListNodesRequest>) -> String {
        let limit = match validate_limit(request.limit, "limit") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };
        let context_keywords = normalize_context_keywords(request.context_keywords.as_deref());
        let query_limit = if context_keywords.is_empty() {
            limit
        } else {
            expanded_limit(limit)
        };

        let find_service = MemoryFindService::new(self.node_store.clone());
        let find_result = match find_service
            .execute(&MemoryFindRequest {
                scope: MemoryScope {
                    tenant_id: None,
                    session_ids: request.session_id.map(|session| vec![session]),
                    tiers: None,
                    from_utc: None,
                    to_utc: None,
                },
                page: MemoryPage {
                    limit: query_limit,
                    cursor: None,
                },
                ..Default::default()
            })
            .await
        {
            Ok(result) => result,
            Err(err) => {
                error!(error = %err, "list_nodes failed");
                return tool_error("ListNodesFailure", &err.to_string());
            }
        };

        let nodes = if context_keywords.is_empty() {
            find_result.nodes.into_iter().take(limit).collect::<Vec<_>>()
        } else {
            filter_nodes_by_context_keywords(&find_result.nodes, &context_keywords, limit)
        };

        to_json_string(json!({
            "retrieved": nodes.len(),
            "nodes": nodes
                .iter()
                .map(sttp_node_to_json)
                .collect::<Vec<_>>()
        }))
    }

    #[tool(
        name = "preview_embedding_migration",
        description = "Preview which nodes would be selected for embedding migration/backfill based on optional filters. Use this before running migration to verify scope and provider availability."
    )]
    async fn preview_embedding_migration(
        &self,
        Parameters(request): Parameters<PreviewEmbeddingMigrationRequest>,
    ) -> String {
        let from_utc = match parse_utc_optional(request.from_utc.as_deref(), "from_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };
        let to_utc = match parse_utc_optional(request.to_utc.as_deref(), "to_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };
        let tiers = request
            .tiers
            .as_ref()
            .map(|values| normalize_tiers(values.as_slice()));
        let sample_limit = match validate_limit(request.sample_limit, "sample_limit") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };
        let max_nodes = match validate_max_nodes(request.max_nodes) {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };

        let filter = EmbeddingMigrationFilter {
            session_id: request.session_id,
            from_utc,
            to_utc,
            tiers,
            has_embedding: request.has_embedding,
            embedding_model: request.embedding_model,
            sync_keys: request.sync_keys,
        };

        match self
            .embedding_migration
            .preview_async(EmbeddingMigrationPreviewRequest {
                filter,
                sample_limit,
                max_nodes,
            })
            .await
        {
            Ok(result) => to_json_string(json!({
                "total_candidates": result.total_candidates,
                "provider_available": result.provider_available,
                "provider_model": result.provider_model,
                "sample": result
                    .sample
                    .iter()
                    .map(|sample| json!({
                        "sync_key": sample.sync_key,
                        "session_id": sample.session_id,
                        "tier": sample.tier,
                        "has_embedding": sample.has_embedding,
                        "embedding_model": sample.embedding_model,
                        "embedding_dimensions": sample.embedding_dimensions,
                        "embedded_at": sample.embedded_at.map(|value| value.to_rfc3339()),
                        "updated_at": sample.updated_at.to_rfc3339(),
                        "context_summary": sample.context_summary,
                    }))
                    .collect::<Vec<_>>(),
            })),
            Err(err) => {
                error!(error = %err, "preview_embedding_migration failed");
                tool_error("MigrationPreviewFailure", &err.to_string())
            }
        }
    }

    #[tool(
        name = "run_embedding_migration",
        description = "Run embedding migration/backfill for selected nodes. Supports dry_run, missing_only mode, and reindex_all mode using the currently configured embedding provider."
    )]
    async fn run_embedding_migration(
        &self,
        Parameters(request): Parameters<RunEmbeddingMigrationRequest>,
    ) -> String {
        let from_utc = match parse_utc_optional(request.from_utc.as_deref(), "from_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };
        let to_utc = match parse_utc_optional(request.to_utc.as_deref(), "to_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };
        let tiers = request
            .tiers
            .as_ref()
            .map(|values| normalize_tiers(values.as_slice()));
        let batch_size = match validate_batch_size(request.batch_size) {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };
        let max_nodes = match validate_max_nodes(request.max_nodes) {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };
        let mode = match parse_migration_mode(request.mode.as_deref()) {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidArgument", &message),
        };

        let filter = EmbeddingMigrationFilter {
            session_id: request.session_id,
            from_utc,
            to_utc,
            tiers,
            has_embedding: request.has_embedding,
            embedding_model: request.embedding_model,
            sync_keys: request.sync_keys,
        };

        match self
            .embedding_migration
            .run_async(EmbeddingMigrationRunRequest {
                filter,
                mode,
                dry_run: request.dry_run,
                batch_size,
                max_nodes,
            })
            .await
        {
            Ok(result) => to_json_string(json!({
                "scanned": result.scanned,
                "selected": result.selected,
                "updated": result.updated,
                "skipped": result.skipped,
                "failed": result.failed,
                "duplicate": result.duplicate,
                "started_at": result.started_at.to_rfc3339(),
                "completed_at": result.completed_at.to_rfc3339(),
                "provider_model": result.provider_model,
                "dry_run": request.dry_run,
                "mode": mode_to_string(mode),
                "failure_reasons": result.failure_reasons,
            })),
            Err(err) => {
                error!(error = %err, "run_embedding_migration failed");
                tool_error("MigrationRunFailure", &err.to_string())
            }
        }
    }

    #[tool(
        name = "get_moods",
        description = "Retrieve AVEC mood presets and optional blend preview to intentionally shift reasoning mode (focused, creative, analytical, exploratory, collaborative, defensive, passive) before memory operations."
    )]
    async fn get_moods(&self, Parameters(request): Parameters<GetMoodsRequest>) -> String {
        let result = self.moods.get(
            request.target_mood.as_deref(),
            request.blend,
            request.current_stability,
            request.current_friction,
            request.current_logic,
            request.current_autonomy,
        );

        let swap_preview = result.swap_preview.as_ref().map(|preview| {
            json!({
                "target_mood": preview.target_mood,
                "blend": preview.blend,
                "current": avec_to_json(preview.current),
                "target": avec_to_json(preview.target),
                "blended": avec_to_json(preview.blended),
            })
        });

        to_json_string(json!({
            "presets": result
                .presets
                .iter()
                .map(|preset| {
                    json!({
                        "name": preset.name,
                        "description": preset.description,
                        "avec": avec_to_json(preset.avec),
                    })
                })
                .collect::<Vec<_>>(),
            "apply_guide": result.apply_guide,
            "swap_preview": swap_preview,
        }))
    }

    #[tool(
        name = "create_monthly_rollup",
        description = "Aggregate many stored nodes into a compact monthly memory checkpoint. Use this to reduce retrieval noise and preserve high-level memory continuity across long timelines."
    )]
    async fn create_monthly_rollup(
        &self,
        Parameters(request): Parameters<CreateMonthlyRollupRequest>,
    ) -> String {
        let start_utc = match parse_utc_required(&request.start_date_utc, "start_date_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };
        let end_utc = match parse_utc_required(&request.end_date_utc, "end_date_utc") {
            Ok(value) => value,
            Err(message) => return tool_error("InvalidDate", &message),
        };

        let mut rollup_request = MonthlyRollupRequest::new(request.session_id, start_utc, end_utc);
        rollup_request.source_session_id = request.source_session_id;
        rollup_request.parent_node_id = request.parent_node_id;
        rollup_request.persist = request.persist;

        let result = self.monthly_rollup.create_async(rollup_request).await;
        if !result.success {
            let message = result
                .error
                .as_deref()
                .unwrap_or("Monthly rollup creation failed.");
            let code = if message.starts_with("InvalidRange") {
                "InvalidRange"
            } else {
                "MonthlyRollupFailure"
            };

            return tool_error(code, message);
        }

        to_json_string(json!({
            "success": result.success,
            "node_id": result.node_id,
            "raw_node": result.raw_node,
            "error": result.error,
            "source_nodes": result.source_nodes,
            "parent_reference": result.parent_reference,
            "user_average": avec_to_json(result.user_average),
            "model_average": avec_to_json(result.model_average),
            "compression_average": avec_to_json(result.compression_average),
            "rho_range": {
                "min": result.rho_range.min,
                "max": result.rho_range.max,
                "average": result.rho_range.average,
            },
            "kappa_range": {
                "min": result.kappa_range.min,
                "max": result.kappa_range.max,
                "average": result.kappa_range.average,
            },
            "psi_range": {
                "min": result.psi_range.min,
                "max": result.psi_range.max,
                "average": result.psi_range.average,
            },
            "rho_bands": {
                "low": result.rho_bands.low,
                "medium": result.rho_bands.medium,
                "high": result.rho_bands.high,
            },
            "kappa_bands": {
                "low": result.kappa_bands.low,
                "medium": result.kappa_bands.medium,
                "high": result.kappa_bands.high,
            },
        }))
    }
}

#[tool_handler]
impl ServerHandler for SttpMcpServer {}

#[derive(Debug, Deserialize, JsonSchema)]
struct CalibrateSessionRequest {
    session_id: String,
    stability: f32,
    friction: f32,
    logic: f32,
    autonomy: f32,
    trigger: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct StoreContextRequest {
    node: String,
    session_id: String,
}

fn default_limit_get_context() -> usize {
    5
}

fn default_blend() -> f32 {
    1.0
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetContextRequest {
    /// Optional session scope. Omit for global retrieval across all sessions.
    #[serde(default)]
    session_id: Option<String>,
    /// Current stability value in [0.0, 1.0].
    stability: f32,
    /// Current friction value in [0.0, 1.0].
    friction: f32,
    /// Current logic value in [0.0, 1.0].
    logic: f32,
    /// Current autonomy value in [0.0, 1.0].
    autonomy: f32,
    /// Maximum number of nodes to return. Required range: 1..=200.
    #[serde(default = "default_limit_get_context")]
    limit: usize,
    /// Optional inclusive UTC lower bound (ISO8601).
    #[serde(default)]
    from_utc: Option<String>,
    /// Optional inclusive UTC upper bound (ISO8601).
    #[serde(default)]
    to_utc: Option<String>,
    /// Optional tier filter (e.g., raw, daily, weekly, monthly).
    #[serde(default)]
    tiers: Option<Vec<String>>,
    /// Optional text keywords for server-side semantic and fuzzy retrieval. Empty arrays are treated as not provided.
    #[serde(default)]
    context_keywords: Option<Vec<String>>,
    /// Optional hybrid resonance weight in [0.0, 1.0]. Only used for semantic retrieval when context_keywords is provided.
    #[serde(default)]
    alpha: Option<f32>,
    /// Optional hybrid semantic weight in [0.0, 1.0]. Only used for semantic retrieval when context_keywords is provided.
    #[serde(default)]
    beta: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListNodesRequest {
    /// Maximum nodes to return. Required range: 1..=200.
    #[serde(default = "default_limit_list_nodes")]
    limit: usize,
    /// Optional session filter. Omit for global listing.
    #[serde(default)]
    session_id: Option<String>,
    /// Optional fuzzy keyword filter over context_summary.
    #[serde(default)]
    context_keywords: Option<Vec<String>>,
}

fn default_limit_list_nodes() -> usize {
    50
}

fn default_sample_limit_preview_migration() -> usize {
    20
}

fn default_batch_size_migration() -> usize {
    100
}

fn default_max_nodes_migration() -> usize {
    5000
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PreviewEmbeddingMigrationRequest {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    from_utc: Option<String>,
    #[serde(default)]
    to_utc: Option<String>,
    #[serde(default)]
    tiers: Option<Vec<String>>,
    #[serde(default)]
    has_embedding: Option<bool>,
    #[serde(default)]
    embedding_model: Option<String>,
    #[serde(default)]
    sync_keys: Option<Vec<String>>,
    #[serde(default = "default_sample_limit_preview_migration")]
    sample_limit: usize,
    #[serde(default = "default_max_nodes_migration")]
    max_nodes: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RunEmbeddingMigrationRequest {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    from_utc: Option<String>,
    #[serde(default)]
    to_utc: Option<String>,
    #[serde(default)]
    tiers: Option<Vec<String>>,
    #[serde(default)]
    has_embedding: Option<bool>,
    #[serde(default)]
    embedding_model: Option<String>,
    #[serde(default)]
    sync_keys: Option<Vec<String>>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default = "default_true")]
    dry_run: bool,
    #[serde(default = "default_batch_size_migration")]
    batch_size: usize,
    #[serde(default = "default_max_nodes_migration")]
    max_nodes: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetMoodsRequest {
    #[serde(default)]
    target_mood: Option<String>,
    #[serde(default = "default_blend")]
    blend: f32,
    #[serde(default)]
    current_stability: Option<f32>,
    #[serde(default)]
    current_friction: Option<f32>,
    #[serde(default)]
    current_logic: Option<f32>,
    #[serde(default)]
    current_autonomy: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateMonthlyRollupRequest {
    session_id: String,
    start_date_utc: String,
    end_date_utc: String,
    #[serde(default)]
    source_session_id: Option<String>,
    #[serde(default)]
    parent_node_id: Option<String>,
    #[serde(default = "default_true")]
    persist: bool,
}

fn default_true() -> bool {
    true
}

pub struct RuntimeSurrealDbClient {
    db: surrealdb::Surreal<Any>,
}

impl RuntimeSurrealDbClient {
    pub async fn connect(
        runtime: &SurrealDbRuntimeOptions,
        user: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self> {
        let db = connect(runtime.endpoint.as_str()).await.with_context(|| {
            format!(
                "failed to connect to SurrealDB endpoint '{}'",
                runtime.endpoint
            )
        })?;

        if runtime.use_remote {
            let username = user
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("root");
            let password = password
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("root");

            db.signin(Root {
                username: username.to_string(),
                password: password.to_string(),
            })
            .await
            .context("failed to authenticate against remote SurrealDB")?;
        }

        db.use_ns(runtime.namespace.as_str())
            .use_db(runtime.database.as_str())
            .await
            .with_context(|| {
                format!(
                    "failed to select namespace '{}' and database '{}'",
                    runtime.namespace, runtime.database
                )
            })?;

        Ok(Self { db })
    }

    fn is_read_query(query: &str) -> bool {
        query
            .trim_start()
            .to_ascii_uppercase()
            .starts_with("SELECT")
    }
}

#[async_trait]
impl SurrealDbClient for RuntimeSurrealDbClient {
    async fn raw_query(
        &self,
        query: &str,
        parameters: locus_core::QueryParams,
    ) -> Result<Vec<Value>> {
        let operation = query
            .split_whitespace()
            .next()
            .unwrap_or("UNKNOWN")
            .to_ascii_uppercase();
        let is_read_query = Self::is_read_query(query);

        let response = if parameters.is_empty() {
            self.db.query(query).await?
        } else {
            self.db.query(query).bind(parameters).await?
        };

        let mut response = match response.check() {
            Ok(value) => value,
            Err(err) => {
                error!(operation = %operation, error = %err, "Surreal query failed");
                return Err(err.into());
            }
        };

        if !is_read_query {
            return Ok(Vec::new());
        }

        if let Ok(rows) = response.take::<Vec<Value>>(0) {
            return Ok(rows);
        }

        if let Ok(Some(row)) = response.take::<Option<Value>>(0) {
            return Ok(vec![row]);
        }

        Ok(Vec::new())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let args = std::env::args().collect::<Vec<_>>();
    let use_in_memory = env_flag("LOCUS_MCP_IN_MEMORY")
        || std::env::var("LOCUS_MCP_STORAGE")
            .map(|value| value.eq_ignore_ascii_case("inmemory"))
            .unwrap_or(false)
        || args
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("--in-memory"));

    let (store, initializer) = if use_in_memory {
        let store = Arc::new(InMemoryNodeStore::new());
        let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
        let node_store: Arc<dyn NodeStore> = store;
        (node_store, initializer)
    } else {
        let settings = load_surreal_settings(&args)?;
        let runtime_args = runtime_args(&args);
        let runtime =
            SurrealDbRuntimeOptions::from_args(&runtime_args, &settings, Some(".locus-mcp"))?;

        let client = Arc::new(
            RuntimeSurrealDbClient::connect(
                &runtime,
                settings.user.as_deref(),
                settings.password.as_deref(),
            )
            .await?,
        );
        let store = Arc::new(SurrealDbNodeStore::new(client));
        let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
        let node_store: Arc<dyn NodeStore> = store;

        info!(
            mode = if runtime.use_remote { "remote" } else { "embedded" },
            endpoint = %runtime.endpoint,
            namespace = %runtime.namespace,
            database = %runtime.database,
            "configured SurrealDB runtime"
        );

        (node_store, initializer)
    };

    initializer.initialize_async().await?;

    let validator: Arc<dyn NodeValidator> = Arc::new(TreeSitterValidator::new());
    let embedding_provider = build_embedding_provider(&args)?;
    let calibration = Arc::new(CalibrationService::new(store.clone()));
    let store_context = match embedding_provider.clone() {
        Some(provider) => Arc::new(StoreContextService::with_embedding_provider(
            store.clone(),
            validator.clone(),
            provider,
        )),
        None => Arc::new(StoreContextService::new(store.clone(), validator.clone())),
    };
    let embedding_migration = Arc::new(EmbeddingMigrationService::new(
        store.clone(),
        embedding_provider.clone(),
    ));
    let moods = Arc::new(MoodCatalogService::new());
    let monthly_rollup = Arc::new(MonthlyRollupService::new(store.clone(), validator));

    let server = SttpMcpServer::new(
        store,
        calibration,
        store_context,
        embedding_migration,
        embedding_provider,
        moods,
        monthly_rollup,
    );

    let running = server
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;
    running.waiting().await?;

    Ok(())
}

fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}

fn load_surreal_settings(args: &[String]) -> Result<SurrealDbSettings> {
    let mut settings = SurrealDbSettings::default();
    settings.endpoints.embedded = Some("surrealkv://data/locus-mcp".to_string());
    settings.database = "locus_mcp".to_string();

    if let Some(value) = env_or_arg(
        "LOCUS_MCP_SURREAL_REMOTE_ENDPOINT",
        args,
        "--remote-endpoint",
    ) {
        settings.endpoints.remote = Some(value);
    }
    if let Some(value) = env_or_arg(
        "LOCUS_MCP_SURREAL_EMBEDDED_ENDPOINT",
        args,
        "--embedded-endpoint",
    ) {
        settings.endpoints.embedded = Some(value);
    }
    if let Some(value) = env_or_arg("LOCUS_MCP_SURREAL_ENDPOINT", args, "--endpoint") {
        settings.endpoints.remote = Some(value.clone());
        settings.endpoints.embedded = Some(value);
    }
    if let Some(value) = env_or_arg("LOCUS_MCP_SURREAL_NAMESPACE", args, "--namespace") {
        settings.namespace = value;
    }
    if let Some(value) = env_or_arg("LOCUS_MCP_SURREAL_DATABASE", args, "--database") {
        settings.database = value;
    }
    if let Some(value) = env_or_arg("LOCUS_MCP_SURREAL_USERNAME", args, "--username") {
        settings.user = Some(value);
    }
    if let Some(value) = env_or_arg("LOCUS_MCP_SURREAL_PASSWORD", args, "--password") {
        settings.password = Some(value);
    }

    Ok(settings)
}

fn env_or_arg(env_key: &str, args: &[String], arg_name: &str) -> Option<String> {
    if let Ok(value) = std::env::var(env_key) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    arg_value(args, arg_name)
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0].eq_ignore_ascii_case(key))
        .map(|window| window[1].clone())
}

fn runtime_args(args: &[String]) -> Vec<String> {
    let mut runtime_args = args.to_vec();
    if env_flag("LOCUS_MCP_REMOTE") && !runtime_args.iter().any(|value| value == "--remote") {
        runtime_args.push("--remote".to_string());
    }
    runtime_args
}

fn build_embedding_provider(args: &[String]) -> Result<Option<Arc<dyn EmbeddingProvider>>> {
    let embeddings_enabled = env_flag("LOCUS_MCP_EMBEDDINGS_ENABLED")
        || args
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("--embeddings-enabled"));

    if !embeddings_enabled {
        return Ok(None);
    }

    let provider_kind_raw = env_or_arg(
        "LOCUS_MCP_EMBEDDINGS_PROVIDER",
        args,
        "--embeddings-provider",
    )
    .unwrap_or_else(|| "ollama".to_string());
    let provider_kind = EmbeddingsProviderKind::parse(&provider_kind_raw).ok_or_else(|| {
        anyhow::anyhow!(
            "unsupported embeddings provider '{}'; expected 'ollama'{}",
            provider_kind_raw,
            if cfg!(feature = "local-embedding") {
                " or 'local'"
            } else {
                ""
            }
        )
    })?;

    let endpoint = env_or_arg(
        "LOCUS_MCP_EMBEDDINGS_ENDPOINT",
        args,
        "--embeddings-endpoint",
    )
    .unwrap_or_else(|| "http://127.0.0.1:11434/api/embeddings".to_string());
    let model = env_or_arg("LOCUS_MCP_EMBEDDINGS_MODEL", args, "--embeddings-model")
        .unwrap_or_else(|| "sttp-encoder".to_string());
    #[cfg(feature = "local-embedding")]
    let repo = env_or_arg("LOCUS_MCP_EMBEDDINGS_REPO", args, "--embeddings-repo")
        .unwrap_or_else(|| "sentence-transformers/all-MiniLM-L6-v2".to_string());

    let provider: Arc<dyn EmbeddingProvider> = match provider_kind {
        EmbeddingsProviderKind::Ollama => {
            info!(
                provider = "ollama",
                endpoint = %endpoint,
                model = %model,
                "auto-embedding enabled for store_context"
            );
            Arc::new(OllamaEmbeddingProvider::new(endpoint, model))
        }
        #[cfg(feature = "local-embedding")]
        EmbeddingsProviderKind::Local => {
            info!(
                provider = "local",
                model = %model,
                repo = %repo,
                "auto-embedding enabled for store_context"
            );
            Arc::new(LocalEmbeddingProvider::new(model, repo)?)
        }
    };

    Ok(Some(provider))
}

fn env_flag(key: &str) -> bool {
    std::env::var(key)
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or(false)
}

fn parse_utc_required(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|_| format!("{field} must be an ISO8601 UTC datetime"))
}

fn parse_utc_optional(value: Option<&str>, field: &str) -> Result<Option<DateTime<Utc>>, String> {
    match value {
        Some(raw) => parse_utc_required(raw, field).map(Some),
        None => Ok(None),
    }
}

fn validate_limit(limit: usize, field: &str) -> Result<usize, String> {
    if (1..=200).contains(&limit) {
        Ok(limit)
    } else {
        Err(format!("{field} must be between 1 and 200"))
    }
}

fn validate_batch_size(batch_size: usize) -> Result<usize, String> {
    if (1..=500).contains(&batch_size) {
        Ok(batch_size)
    } else {
        Err("batch_size must be between 1 and 500".to_string())
    }
}

fn validate_max_nodes(max_nodes: usize) -> Result<usize, String> {
    if (1..=50000).contains(&max_nodes) {
        Ok(max_nodes)
    } else {
        Err("max_nodes must be between 1 and 50000".to_string())
    }
}

fn parse_migration_mode(value: Option<&str>) -> Result<EmbeddingMigrationMode, String> {
    match value
        .unwrap_or("missing_only")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "missing_only" => Ok(EmbeddingMigrationMode::MissingOnly),
        "reindex_all" => Ok(EmbeddingMigrationMode::ReindexAll),
        _ => Err("mode must be one of: missing_only, reindex_all".to_string()),
    }
}

fn mode_to_string(mode: EmbeddingMigrationMode) -> &'static str {
    match mode {
        EmbeddingMigrationMode::MissingOnly => "missing_only",
        EmbeddingMigrationMode::ReindexAll => "reindex_all",
    }
}

fn expanded_limit(limit: usize) -> usize {
    limit.saturating_mul(5).clamp(1, 200)
}

fn normalize_tiers(tiers: &[String]) -> Vec<String> {
    tiers
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
}

fn normalize_context_keywords(keywords: Option<&[String]>) -> Vec<String> {
    keywords
        .unwrap_or(&[])
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
}

fn context_keyword_score(node: &locus_core::SttpNode, keywords: &[String]) -> usize {
    let summary = node
        .context_summary
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let session_id = node.session_id.to_ascii_lowercase();

    keywords
        .iter()
        .filter(|keyword| {
            let needle = keyword.as_str();
            summary.contains(needle) || session_id.contains(needle)
        })
        .count()
}

fn filter_nodes_by_context_keywords(
    nodes: &[locus_core::SttpNode],
    keywords: &[String],
    limit: usize,
) -> Vec<locus_core::SttpNode> {
    let mut scored = nodes
        .iter()
        .filter_map(|node| {
            let score = context_keyword_score(node, keywords);
            if score == 0 {
                None
            } else {
                Some((score, node.timestamp, node.clone()))
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, node)| node)
        .collect::<Vec<_>>()
}

fn to_json_string(value: Value) -> String {
    match serde_json::to_string(&value) {
        Ok(serialized) => serialized,
        Err(err) => tool_error("SerializationFailure", &err.to_string()),
    }
}

fn tool_error(code: &str, message: &str) -> String {
    to_json_string(json!({
        "error": {
            "code": code,
            "message": message,
        }
    }))
}

fn avec_to_json(avec: locus_core::AvecState) -> Value {
    json!({
        "stability": avec.stability,
        "friction": avec.friction,
        "logic": avec.logic,
        "autonomy": avec.autonomy,
        "psi": avec.psi(),
    })
}

fn sttp_node_to_json(node: &locus_core::SttpNode) -> Value {
    json!({
        "raw": node.raw,
        "session_id": node.session_id,
        "tier": node.tier,
        "timestamp": node.timestamp.to_rfc3339(),
        "compression_depth": node.compression_depth,
        "parent_node_id": node.parent_node_id,
        "sync_key": node.sync_key,
        "updated_at": node.updated_at.to_rfc3339(),
        "source_metadata": node.source_metadata,
        "context_summary": node.context_summary,
        "has_embedding": node.embedding.as_ref().map(|values| !values.is_empty()).unwrap_or(false),
        "embedding_model": node.embedding_model,
        "embedding_dimensions": node.embedding_dimensions,
        "embedded_at": node.embedded_at.map(|value| value.to_rfc3339()),
        "user_avec": avec_to_json(node.user_avec),
        "model_avec": avec_to_json(node.model_avec),
        "compression_avec": node.compression_avec.map(avec_to_json),
        "rho": node.rho,
        "kappa": node.kappa,
        "psi": node.psi,
    })
}

