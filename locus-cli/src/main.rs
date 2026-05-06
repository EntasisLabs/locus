use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use locus_core_rs::domain::models::{AvecState, MonthlyRollupRequest, SttpNode};
use locus_core_rs::{
    CalibrationService, InMemoryNodeStore, MonthlyRollupService, MoodCatalogService, NodeStore,
    NodeStoreInitializer, NodeValidator, QueryParams,
    StoreContextService, SurrealDbClient, SurrealDbEndpointsSettings, SurrealDbNodeStore,
    SurrealDbRuntimeOptions, SurrealDbSettings, TreeSitterValidator,
};
use locus_sdk::application::memory_find::MemoryFindService;
use locus_sdk::application::memory_recall::MemoryRecallService;
use locus_sdk::domain::memory::{MemoryFindRequest, MemoryPage, MemoryRecallRequest, MemoryScope};
use serde_json::{Value, json};
use surrealdb::engine::any::{Any, connect};
use surrealdb::opt::auth::Root;

const DEFAULT_TENANT: &str = "default";
const TENANT_SCOPE_PREFIX: &str = "tenant:";
const TENANT_SCOPE_SEPARATOR: &str = "::session:";

#[derive(Copy, Clone, Debug, ValueEnum)]
enum StorageMode {
    InMemory,
    Surreal,
}

#[derive(Parser, Debug)]
#[command(name = "locus-cli", version, about = "SDK-backed CLI for Locus memory operations")]
struct Cli {
    #[arg(long, env = "LOCUS_STORAGE", default_value = "surreal")]
    storage: StorageMode,

    #[arg(long, env = "LOCUS_TENANT_ID", help = "Optional tenant ID")]
    tenant_id: Option<String>,

    #[arg(long, env = "LOCUS_REMOTE", default_value_t = false)]
    remote: bool,

    #[arg(long, env = "LOCUS_ROOT_DIR_NAME", default_value = ".locus-cli")]
    root_dir_name: String,

    #[arg(long, env = "LOCUS_SURREAL_ENDPOINT")]
    surreal_endpoint: Option<String>,

    #[arg(long, env = "LOCUS_SURREAL_REMOTE_ENDPOINT")]
    surreal_remote_endpoint: Option<String>,

    #[arg(long, env = "LOCUS_SURREAL_EMBEDDED_ENDPOINT")]
    surreal_embedded_endpoint: Option<String>,

    #[arg(long, env = "LOCUS_SURREAL_NAMESPACE", default_value = "entasis")]
    surreal_namespace: String,

    #[arg(long, env = "LOCUS_SURREAL_DATABASE", default_value = "locus_cli")]
    surreal_database: String,

    #[arg(long, env = "LOCUS_SURREAL_USERNAME")]
    surreal_username: Option<String>,

    #[arg(long, env = "LOCUS_SURREAL_PASSWORD")]
    surreal_password: Option<String>,

    #[arg(long, help = "Pretty-print JSON output")]
    pretty: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Health,
    Calibrate {
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        stability: f32,
        #[arg(long)]
        friction: f32,
        #[arg(long)]
        logic: f32,
        #[arg(long)]
        autonomy: f32,
        #[arg(long, default_value = "manual")]
        trigger: String,
    },
    Store {
        #[arg(long)]
        session_id: String,
        #[arg(long, help = "Path to a file containing one STTP node")]
        node_file: PathBuf,
    },
    Context {
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        stability: f32,
        #[arg(long)]
        friction: f32,
        #[arg(long)]
        logic: f32,
        #[arg(long)]
        autonomy: f32,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        from_utc: Option<String>,
        #[arg(long)]
        to_utc: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tiers: Vec<String>,
        #[arg(long)]
        query_text: Option<String>,
        #[arg(long)]
        alpha: Option<f32>,
        #[arg(long)]
        beta: Option<f32>,
    },
    Nodes {
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        session_id: Option<String>,
    },
    Moods {
        #[arg(long)]
        target_mood: Option<String>,
        #[arg(long)]
        blend: Option<f32>,
        #[arg(long)]
        current_stability: Option<f32>,
        #[arg(long)]
        current_friction: Option<f32>,
        #[arg(long)]
        current_logic: Option<f32>,
        #[arg(long)]
        current_autonomy: Option<f32>,
    },
    Rollup {
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        start_date_utc: String,
        #[arg(long)]
        end_date_utc: String,
        #[arg(long)]
        source_session_id: Option<String>,
        #[arg(long)]
        parent_node_id: Option<String>,
        #[arg(long)]
        persist: Option<bool>,
        #[arg(long)]
        limit: Option<usize>,
    },
}

struct Services {
    calibration: CalibrationService,
    store_context: StoreContextService,
    memory_find: MemoryFindService,
    memory_recall: MemoryRecallService,
    moods: MoodCatalogService,
    monthly_rollup: MonthlyRollupService,
    storage_mode: &'static str,
    storage_endpoint: Option<String>,
    storage_namespace: Option<String>,
    storage_database: Option<String>,
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
            let username = user.filter(|v| !v.trim().is_empty()).unwrap_or("root");
            let password = password.filter(|v| !v.trim().is_empty()).unwrap_or("root");

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
    async fn raw_query(&self, query: &str, parameters: QueryParams) -> Result<Vec<Value>> {
        let is_read_query = Self::is_read_query(query);

        let response = if parameters.is_empty() {
            self.db.query(query).await?
        } else {
            self.db.query(query).bind(parameters).await?
        };

        let mut response = response.check()?;

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
    let cli = Cli::parse();
    let tenant = resolve_tenant(cli.tenant_id.as_deref())?;
    let services = build_services(&cli).await?;

    let output = match cli.command {
        Commands::Health => {
            json!({
                "status": "ok",
                "transport": "sdk-core",
                "storage": {
                    "mode": services.storage_mode,
                    "endpoint": services.storage_endpoint,
                    "namespace": services.storage_namespace,
                    "database": services.storage_database,
                }
            })
        }
        Commands::Calibrate {
            session_id,
            stability,
            friction,
            logic,
            autonomy,
            trigger,
        } => {
            let session_id = scope_session_id(&tenant, &session_id);
            let result = services
                .calibration
                .calibrate_async(
                    &session_id,
                    stability,
                    friction,
                    logic,
                    autonomy,
                    &trigger,
                )
                .await?;

            json!({
                "previousAvec": avec_to_json(result.previous_avec),
                "delta": result.delta,
                "driftClassification": format!("{:?}", result.drift_classification),
                "trigger": result.trigger,
                "triggerHistory": result.trigger_history,
                "isFirstCalibration": result.is_first_calibration,
            })
        }
        Commands::Store {
            session_id,
            node_file,
        } => {
            let session_id = scope_session_id(&tenant, &session_id);
            let node = fs::read_to_string(&node_file)
                .with_context(|| format!("failed to read node file: {}", node_file.display()))?;

            if node.trim().is_empty() {
                bail!("node file is empty");
            }

            let result = services.store_context.store_async(&node, &session_id).await;
            json!({
                "nodeId": result.node_id,
                "psi": result.psi,
                "valid": result.valid,
                "validationError": result.validation_error,
            })
        }
        Commands::Context {
            session_id,
            stability,
            friction,
            logic,
            autonomy,
            limit,
            from_utc,
            to_utc,
            tiers,
            query_text,
            alpha,
            beta,
        } => {
            let session_id = scope_session_id(&tenant, &session_id);
            let from_utc = parse_utc_optional(from_utc.as_deref(), "from_utc")?;
            let to_utc = parse_utc_optional(to_utc.as_deref(), "to_utc")?;
            let tiers = normalize_tiers(tiers);

            let request = MemoryRecallRequest {
                scope: MemoryScope {
                    tenant_id: None,
                    session_ids: Some(vec![session_id]),
                    tiers,
                    from_utc,
                    to_utc,
                },
                page: MemoryPage {
                    limit: limit.unwrap_or(5),
                    cursor: None,
                },
                scoring: locus_sdk::domain::memory::MemoryScoring {
                    alpha: alpha.unwrap_or(0.7),
                    beta: beta.unwrap_or(0.3),
                    ..Default::default()
                },
                current_avec: Some(AvecState {
                    stability,
                    friction,
                    logic,
                    autonomy,
                }),
                query_text,
                query_embedding: None,
                ..Default::default()
            };

            let result = services.memory_recall.execute(&request).await?;
            let nodes = normalize_nodes_for_tenant(result.nodes, &tenant);

            json!({
                "nodes": nodes.iter().map(sttp_node_to_json).collect::<Vec<_>>(),
                "retrieved": nodes.len(),
                "psiRange": {
                    "min": result.psi_range.min,
                    "max": result.psi_range.max,
                    "average": result.psi_range.average,
                },
                "retrievalPath": format!("{:?}", result.retrieval_path),
                "hasMore": result.has_more,
                "nextCursor": result.next_cursor,
            })
        }
        Commands::Nodes { limit, session_id } => {
            let requested_limit = limit.unwrap_or(50).clamp(1, 200);

            let scoped_session = session_id.as_deref().map(|id| scope_session_id(&tenant, id));
            let result = services
                .memory_find
                .execute(&MemoryFindRequest {
                    scope: MemoryScope {
                        session_ids: scoped_session.map(|id| vec![id]),
                        ..Default::default()
                    },
                    page: MemoryPage {
                        limit: if session_id.is_some() {
                            requested_limit
                        } else {
                            (requested_limit * 4).clamp(1, 200)
                        },
                        cursor: None,
                    },
                    ..Default::default()
                })
                .await?;

            let mut nodes = normalize_nodes_for_tenant(result.nodes, &tenant);
            nodes.truncate(requested_limit);

            json!({
                "nodes": nodes.iter().map(sttp_node_to_json).collect::<Vec<_>>(),
                "retrieved": nodes.len(),
            })
        }
        Commands::Moods {
            target_mood,
            blend,
            current_stability,
            current_friction,
            current_logic,
            current_autonomy,
        } => {
            let result = services.moods.get(
                target_mood.as_deref(),
                blend.unwrap_or(1.0),
                current_stability,
                current_friction,
                current_logic,
                current_autonomy,
            );

            json!({
                "presets": result.presets.iter().map(|preset| json!({
                    "name": preset.name,
                    "description": preset.description,
                    "avec": avec_to_json(preset.avec),
                })).collect::<Vec<_>>(),
                "applyGuide": result.apply_guide,
                "swapPreview": result.swap_preview.map(|preview| json!({
                    "targetMood": preview.target_mood,
                    "blend": preview.blend,
                    "current": avec_to_json(preview.current),
                    "target": avec_to_json(preview.target),
                    "blended": avec_to_json(preview.blended),
                })),
            })
        }
        Commands::Rollup {
            session_id,
            start_date_utc,
            end_date_utc,
            source_session_id,
            parent_node_id,
            persist,
            limit,
        } => {
            let session_id = scope_session_id(&tenant, &session_id);
            let source_session_id = source_session_id.map(|id| scope_session_id(&tenant, &id));
            let request = MonthlyRollupRequest {
                session_id,
                start_utc: parse_utc_required(&start_date_utc, "start_date_utc")?,
                end_utc: parse_utc_required(&end_date_utc, "end_date_utc")?,
                source_session_id,
                parent_node_id,
                limit: limit.unwrap_or(5000),
                persist: persist.unwrap_or(true),
            };

            let result = services.monthly_rollup.create_async(request).await;
            json!({
                "success": result.success,
                "nodeId": result.node_id,
                "rawNode": result.raw_node,
                "error": result.error,
                "sourceNodes": result.source_nodes,
                "parentReference": result.parent_reference,
                "userAverage": avec_to_json(result.user_average),
                "modelAverage": avec_to_json(result.model_average),
                "compressionAverage": avec_to_json(result.compression_average),
                "rhoRange": {
                    "min": result.rho_range.min,
                    "max": result.rho_range.max,
                    "average": result.rho_range.average,
                },
                "kappaRange": {
                    "min": result.kappa_range.min,
                    "max": result.kappa_range.max,
                    "average": result.kappa_range.average,
                },
                "psiRange": {
                    "min": result.psi_range.min,
                    "max": result.psi_range.max,
                    "average": result.psi_range.average,
                },
                "rhoBands": {
                    "low": result.rho_bands.low,
                    "medium": result.rho_bands.medium,
                    "high": result.rho_bands.high,
                },
                "kappaBands": {
                    "low": result.kappa_bands.low,
                    "medium": result.kappa_bands.medium,
                    "high": result.kappa_bands.high,
                },
            })
        }
    };

    if cli.pretty {
        println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .context("failed to render pretty JSON output")?
        );
    } else {
        println!("{}", serde_json::to_string(&output)?);
    }

    Ok(())
}

async fn build_services(cli: &Cli) -> Result<Services> {
    let (store, initializer, storage_mode, storage_endpoint, storage_namespace, storage_database) =
        match cli.storage {
            StorageMode::InMemory => {
                let store = Arc::new(InMemoryNodeStore::new());
                let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
                let node_store: Arc<dyn NodeStore> = store;
                (
                    node_store,
                    initializer,
                    "in-memory",
                    None,
                    None,
                    None,
                )
            }
            StorageMode::Surreal => {
                let settings = surreal_settings_from_cli(cli);
                let runtime = surreal_runtime_from_cli(cli, &settings)?;

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

                (
                    node_store,
                    initializer,
                    if runtime.use_remote {
                        "surreal-remote"
                    } else {
                        "surreal-embedded"
                    },
                    Some(runtime.endpoint),
                    Some(runtime.namespace),
                    Some(runtime.database),
                )
            }
        };

    initializer.initialize_async().await?;

    let validator: Arc<dyn NodeValidator> = Arc::new(TreeSitterValidator::new());

    Ok(Services {
        calibration: CalibrationService::new(store.clone()),
        store_context: StoreContextService::new(store.clone(), validator.clone()),
        memory_find: MemoryFindService::new(store.clone()),
        memory_recall: MemoryRecallService::new(store.clone()),
        moods: MoodCatalogService::new(),
        monthly_rollup: MonthlyRollupService::new(store, validator),
        storage_mode,
        storage_endpoint,
        storage_namespace,
        storage_database,
    })
}

fn surreal_settings_from_cli(cli: &Cli) -> SurrealDbSettings {
    let mut settings = SurrealDbSettings {
        endpoints: SurrealDbEndpointsSettings {
            embedded: Some(
                cli.surreal_embedded_endpoint
                    .clone()
                    .unwrap_or_else(|| "surrealkv://data/locus-cli".to_string()),
            ),
            remote: cli.surreal_remote_endpoint.clone(),
        },
        namespace: cli.surreal_namespace.clone(),
        database: cli.surreal_database.clone(),
        user: cli.surreal_username.clone(),
        password: cli.surreal_password.clone(),
    };

    if let Some(endpoint) = cli
        .surreal_endpoint
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        settings.endpoints.embedded = Some(endpoint.to_string());
        settings.endpoints.remote = Some(endpoint.to_string());
    }

    settings
}

fn surreal_runtime_from_cli(cli: &Cli, settings: &SurrealDbSettings) -> Result<SurrealDbRuntimeOptions> {
    let mut args = Vec::new();
    if cli.remote {
        args.push("--remote".to_string());
    }

    SurrealDbRuntimeOptions::from_args(&args, settings, Some(&cli.root_dir_name))
}

fn normalize_tiers(tiers: Vec<String>) -> Option<Vec<String>> {
    let tiers = tiers
        .into_iter()
        .map(|tier| tier.trim().to_ascii_lowercase())
        .filter(|tier| !tier.is_empty())
        .collect::<Vec<_>>();

    if tiers.is_empty() {
        None
    } else {
        Some(tiers)
    }
}

fn resolve_tenant(value: Option<&str>) -> Result<String> {
    match value.and_then(normalize_tenant_value) {
        Some(tenant) => Ok(tenant),
        None => {
            if value.is_some() {
                bail!("tenant id can only contain letters, digits, '-' or '_'");
            }
            Ok(DEFAULT_TENANT.to_string())
        }
    }
}

fn normalize_tenant_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.to_ascii_lowercase();
    if normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Some(normalized)
    } else {
        None
    }
}

fn scope_session_id(tenant: &str, session_id: &str) -> String {
    if tenant == DEFAULT_TENANT {
        session_id.to_string()
    } else {
        format!("{TENANT_SCOPE_PREFIX}{tenant}{TENANT_SCOPE_SEPARATOR}{session_id}")
    }
}

fn parse_scoped_session_id(session_id: &str) -> Option<(&str, &str)> {
    let remainder = session_id.strip_prefix(TENANT_SCOPE_PREFIX)?;
    remainder.split_once(TENANT_SCOPE_SEPARATOR)
}

fn session_belongs_to_tenant(session_id: &str, tenant: &str) -> bool {
    match parse_scoped_session_id(session_id) {
        Some((scoped_tenant, _)) => scoped_tenant == tenant,
        None => tenant == DEFAULT_TENANT,
    }
}

fn display_session_id(session_id: &str) -> String {
    match parse_scoped_session_id(session_id) {
        Some((_, base_session_id)) => base_session_id.to_string(),
        None => session_id.to_string(),
    }
}

fn normalize_nodes_for_tenant(nodes: Vec<SttpNode>, tenant: &str) -> Vec<SttpNode> {
    nodes
        .into_iter()
        .filter_map(|mut node| {
            if !session_belongs_to_tenant(&node.session_id, tenant) {
                return None;
            }
            node.session_id = display_session_id(&node.session_id);
            Some(node)
        })
        .collect()
}

fn avec_to_json(avec: AvecState) -> Value {
    json!({
        "stability": avec.stability,
        "friction": avec.friction,
        "logic": avec.logic,
        "autonomy": avec.autonomy,
        "psi": avec.psi(),
    })
}

fn sttp_node_to_json(node: &SttpNode) -> Value {
    json!({
        "raw": node.raw,
        "sessionId": node.session_id,
        "tier": node.tier,
        "timestamp": node.timestamp.to_rfc3339(),
        "compressionDepth": node.compression_depth,
        "parentNodeId": node.parent_node_id,
        "syncKey": node.sync_key,
        "updatedAt": node.updated_at.to_rfc3339(),
        "contextSummary": node.context_summary,
        "embeddingModel": node.embedding_model,
        "embeddingDimensions": node.embedding_dimensions,
        "embeddedAt": node.embedded_at.map(|v| v.to_rfc3339()),
        "userAvec": avec_to_json(node.user_avec),
        "modelAvec": avec_to_json(node.model_avec),
        "compressionAvec": node.compression_avec.map(avec_to_json),
        "rho": node.rho,
        "kappa": node.kappa,
        "psi": node.psi,
    })
}

fn parse_utc_required(value: &str, field: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|_| anyhow!("{field} must be an ISO8601 UTC datetime"))
}

fn parse_utc_optional(value: Option<&str>, field: &str) -> Result<Option<DateTime<Utc>>> {
    match value {
        Some(raw) => parse_utc_required(raw, field).map(Some),
        None => Ok(None),
    }
}
