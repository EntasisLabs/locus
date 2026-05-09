//! `locus-mcp` binary.
//!
//! Exposes STTP memory operations over the Model Context Protocol (MCP)
//! for assistant and agent runtimes.

use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::domain::contracts::EmbeddingProvider;
use locus_core_rs::{
    CalibrationService, EmbeddingMigrationService, InMemoryNodeStore, MonthlyRollupService,
    MoodCatalogService, NodeStore, NodeStoreInitializer, NodeValidator, StoreContextService,
    SttpNodeParser, SurrealDbNodeStore, SurrealDbRuntimeOptions, TreeSitterValidator,
};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

mod composition;
mod shared;
mod tools;

use composition::{
    RuntimeSurrealDbClient, build_embedding_provider, init_logging, load_surreal_settings,
    resolve_parser_profile, runtime_args,
};

pub(crate) use shared::{
    avec_to_json, expanded_limit, filter_nodes_by_context_keywords, infer_store_error_code,
    mode_to_string, normalize_context_keywords, normalize_tiers, parse_migration_mode,
    parse_utc_optional, parse_utc_required, schema_first_guidance_payload,
    strict_typed_ir_profile_name, sttp_node_to_json, to_json_string, tool_error,
    validate_batch_size, validate_limit, validate_max_nodes,
};

#[derive(Clone)]
pub(crate) struct SttpMcpServer {
    pub(crate) node_store: Arc<dyn NodeStore>,
    pub(crate) calibration: Arc<CalibrationService>,
    pub(crate) store_context: Arc<StoreContextService>,
    pub(crate) embedding_migration: Arc<EmbeddingMigrationService>,
    pub(crate) embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    pub(crate) moods: Arc<MoodCatalogService>,
    pub(crate) monthly_rollup: Arc<MonthlyRollupService>,
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

    pub(crate) async fn embed_context_keywords(&self, keywords: &[String]) -> Option<Vec<f32>> {
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
        name = "get_schema",
        description = "Get a canonical example of what an STTP node should look like before storage."
    )]
    async fn get_schema(&self) -> String {
        tools::get_schema::execute().await
    }

    #[tool(
        name = "calibrate_session",
        description = "Call this at session start and after heavy reasoning work to measure current AVEC drift. Use it to compare your current cognitive state against prior calibration for the same session before storing or retrieving memory. On first calibration, name the session id something similar to the topic of the conversation if no session id was provided by user."
    )]
    async fn calibrate_session(
        &self,
        Parameters(request): Parameters<CalibrateSessionRequest>,
    ) -> String {
        tools::calibrate_session::execute(self, request).await
    }

    #[tool(
        name = "store_context",
        description = "Call this when context should be preserved to memory. Store a complete valid STTP node so future retrieval can rehydrate prior reasoning state, decisions, and confidence signals. If no session id provided by user, use something that the user can semantically relate to the conversation for better retrieval."
    )]
    async fn store_context(&self, Parameters(request): Parameters<StoreContextRequest>) -> String {
        tools::store_context::execute(self, request).await
    }

    #[tool(
        name = "get_context",
        description = "Primary memory retrieval tool. MUST USE ANYTIME USER ASKS SOMETHING ABOUT REMEMBERING OR MEMORY RELATED INQUIERIES. Returns top resonant memory nodes for the provided AVEC state. Optional context_keywords enables server-side semantic retrieval (with internal embedding generation); keyword fallback is only used when semantic retrieval returns no nodes (or embeddings are unavailable). If session_id is omitted, retrieval is global across sessions. Use list_nodes for inventory when no results comeback after user prompts for memory retrieval."
    )]
    async fn get_context(&self, Parameters(request): Parameters<GetContextRequest>) -> String {
        tools::get_context::execute(self, request).await
    }

    #[tool(
        name = "list_nodes",
        description = "Memory inventory tool. Lists stored nodes newest-first (global when session_id is omitted). Optional context_keywords performs fuzzy and semantic filtering against context_summary for fast discovery. Unlike get_context, list_nodes does not perform AVEC resonance ranking."
    )]
    async fn list_nodes(&self, Parameters(request): Parameters<ListNodesRequest>) -> String {
        tools::list_nodes::execute(self, request).await
    }

    #[tool(
        name = "preview_embedding_migration",
        description = "Preview which nodes would be selected for embedding migration/backfill based on optional filters. Use this before running migration to verify scope and provider availability."
    )]
    async fn preview_embedding_migration(
        &self,
        Parameters(request): Parameters<PreviewEmbeddingMigrationRequest>,
    ) -> String {
        tools::preview_embedding_migration::execute(self, request).await
    }

    #[tool(
        name = "run_embedding_migration",
        description = "Run embedding migration/backfill for selected nodes. Supports dry_run, missing_only mode, and reindex_all mode using the currently configured embedding provider."
    )]
    async fn run_embedding_migration(
        &self,
        Parameters(request): Parameters<RunEmbeddingMigrationRequest>,
    ) -> String {
        tools::run_embedding_migration::execute(self, request).await
    }

    #[tool(
        name = "get_moods",
        description = "Retrieve AVEC mood presets and optional blend preview to intentionally shift reasoning mode (focused, creative, analytical, exploratory, collaborative, defensive, passive) before memory operations. Help maintain coherence and tone. USE WHEN ASKED TO STORE OR RETRIEVE MEMORY WITHOUT INITIAL AVEC CONFIG."
    )]
    async fn get_moods(&self, Parameters(request): Parameters<GetMoodsRequest>) -> String {
        tools::get_moods::execute(self, request).await
    }

    #[tool(
        name = "create_monthly_rollup",
        description = "Aggregate many stored nodes into a compact monthly memory checkpoint. Use this to reduce retrieval noise and preserve high-level memory continuity across long timelines."
    )]
    async fn create_monthly_rollup(
        &self,
        Parameters(request): Parameters<CreateMonthlyRollupRequest>,
    ) -> String {
        tools::create_monthly_rollup::execute(self, request).await
    }
}

#[tool_handler]
impl ServerHandler for SttpMcpServer {}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CalibrateSessionRequest {
    session_id: String,
    stability: f32,
    friction: f32,
    logic: f32,
    autonomy: f32,
    trigger: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct StoreContextRequest {
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
pub(crate) struct GetContextRequest {
    #[serde(default)]
    session_id: Option<String>,
    stability: f32,
    friction: f32,
    logic: f32,
    autonomy: f32,
    #[serde(default = "default_limit_get_context")]
    limit: usize,
    #[serde(default)]
    from_utc: Option<String>,
    #[serde(default)]
    to_utc: Option<String>,
    #[serde(default)]
    tiers: Option<Vec<String>>,
    #[serde(default)]
    context_keywords: Option<Vec<String>>,
    #[serde(default)]
    alpha: Option<f32>,
    #[serde(default)]
    beta: Option<f32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ListNodesRequest {
    #[serde(default = "default_limit_list_nodes")]
    limit: usize,
    #[serde(default)]
    session_id: Option<String>,
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
pub(crate) struct PreviewEmbeddingMigrationRequest {
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
pub(crate) struct RunEmbeddingMigrationRequest {
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
pub(crate) struct GetMoodsRequest {
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
pub(crate) struct CreateMonthlyRollupRequest {
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

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let args = std::env::args().collect::<Vec<_>>();
    let use_in_memory = std::env::var("LOCUS_MCP_IN_MEMORY")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or(false)
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
        let runtime = SurrealDbRuntimeOptions::from_args(&runtime_args, &settings, Some(".locus-mcp"))?;

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

        tracing::info!(
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
    let parse_profile = resolve_parser_profile(&args)?;
    let parser = SttpNodeParser::with_profile(parse_profile);
    tracing::info!(parse_profile = ?parse_profile, "configured STTP parser profile for store_context");

    let calibration = Arc::new(CalibrationService::new(store.clone()));
    let store_context = match embedding_provider.clone() {
        Some(provider) => Arc::new(StoreContextService::with_embedding_provider(
            store.clone(),
            validator.clone(),
            provider,
            parser,
        )),
        None => Arc::new(StoreContextService::new(store.clone(), validator.clone(), parser)),
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
