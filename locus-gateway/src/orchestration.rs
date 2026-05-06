use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::http::HeaderValue;
use locus_core_rs::application::services::{
    CalibrationService, ContextQueryService, MonthlyRollupService,
    MoodCatalogService, RekeyScopeService, StoreContextService,
};
use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::{
    EmbeddingProvider, NodeStore, NodeStoreInitializer, NodeValidator,
};
use locus_core_rs::storage::{
    InMemoryNodeStore, SurrealDbEndpointsSettings, SurrealDbNodeStore, SurrealDbRuntimeOptions,
    SurrealDbSettings,
};
#[cfg(feature = "local-embedding")]
use locus_sdk::infrastructure::embeddings::LocalEmbeddingProvider;
use locus_sdk::infrastructure::embeddings::OllamaEmbeddingProvider;
use tracing::{error, info};

use crate::app_state::AppState;
use crate::gateway_args::{EmbeddingsProviderKind, GatewayArgs, GatewayBackend};
use crate::http_models::CorsAllowedOrigins;
use crate::providers::{AvecScorer, OllamaAvecScorer};
use crate::surreal_client::RuntimeSurrealDbClient;

pub(crate) async fn build_state(args: &GatewayArgs) -> Result<AppState> {
    build_state_with_backend(&args.backend, Some(args)).await
}

#[cfg(test)]
pub(crate) async fn build_in_memory_state() -> Result<AppState> {
    build_in_memory_state_with_args(None).await
}

pub(crate) fn parse_cors_allowed_origins(value: &str) -> Result<CorsAllowedOrigins> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(
            "CORS allowed origins cannot be empty when CORS is enabled"
        ));
    }

    if trimmed == "*" {
        return Ok(CorsAllowedOrigins::Any);
    }

    let mut origins = Vec::new();
    for origin in trimmed
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let header = HeaderValue::from_str(origin)
            .map_err(|_| anyhow!("Invalid CORS origin value: {origin}"))?;
        origins.push(header);
    }

    if origins.is_empty() {
        return Err(anyhow!(
            "CORS allowed origins must include at least one origin or '*'"
        ));
    }

    Ok(CorsAllowedOrigins::Explicit(origins))
}

pub(crate) async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        error!(error = %err, "Failed waiting for ctrl_c signal");
    }
}

async fn build_state_with_backend(
    backend: &GatewayBackend,
    options: Option<&GatewayArgs>,
) -> Result<AppState> {
    match backend {
        GatewayBackend::InMemory => build_in_memory_state_with_args(options).await,
        GatewayBackend::Surreal => {
            let options = options.ok_or_else(|| {
                anyhow!("Surreal backend selected, but no gateway runtime options were provided.")
            })?;
            build_surreal_state(options).await
        }
    }
}

async fn build_in_memory_state_with_args(args: Option<&GatewayArgs>) -> Result<AppState> {
    let store = Arc::new(InMemoryNodeStore::new());

    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer.initialize_async().await?;

    let store_trait: Arc<dyn NodeStore> = store;
    let validator: Arc<dyn NodeValidator> = Arc::new(TreeSitterValidator);
    let embedding_provider = build_embedding_provider(args)?;
    let avec_scorer = build_avec_scorer(args);

    Ok(build_services(
        store_trait,
        validator,
        embedding_provider,
        avec_scorer,
    ))
}

fn build_services(
    store_trait: Arc<dyn NodeStore>,
    validator: Arc<dyn NodeValidator>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    avec_scorer: Option<Arc<dyn AvecScorer>>,
) -> AppState {
    let store_context = match embedding_provider.as_ref() {
        Some(provider) => Arc::new(StoreContextService::with_embedding_provider(
            store_trait.clone(),
            validator.clone(),
            provider.clone(),
        )),
        None => Arc::new(StoreContextService::new(
            store_trait.clone(),
            validator.clone(),
        )),
    };

    AppState {
        node_store: store_trait.clone(),
        embedding_provider: embedding_provider.clone(),
        avec_scorer,
        calibration: Arc::new(CalibrationService::new(store_trait.clone())),
        context_query: Arc::new(ContextQueryService::new(store_trait.clone())),
        mood_catalog: Arc::new(MoodCatalogService::new()),
        store_context,
        monthly_rollup: Arc::new(MonthlyRollupService::new(store_trait.clone(), validator)),
        rekey_scope: Arc::new(RekeyScopeService::new(store_trait)),
    }
}

async fn build_surreal_state(args: &GatewayArgs) -> Result<AppState> {
    let mut settings = SurrealDbSettings::default();
    settings.endpoints = SurrealDbEndpointsSettings {
        embedded: args
            .surreal_embedded_endpoint
            .clone()
            .or(settings.endpoints.embedded),
        remote: args
            .surreal_remote_endpoint
            .clone()
            .or(settings.endpoints.remote),
    };
    settings.namespace = args.surreal_namespace.clone();
    settings.database = args.surreal_database.clone();
    settings.user = Some(args.surreal_user.clone());
    settings.password = Some(args.surreal_password.clone());

    let mut runtime_args = Vec::new();
    if args.remote {
        runtime_args.push("--remote".to_string());
    }

    let runtime = SurrealDbRuntimeOptions::from_args(
        &runtime_args,
        &settings,
        Some(args.root_dir_name.as_str()),
    )?;

    info!(
        backend = "surreal",
        root_dir = runtime.root_dir,
        mode = if runtime.use_remote {
            "remote"
        } else {
            "embedded"
        },
        endpoint = runtime.endpoint,
        namespace = runtime.namespace,
        database = runtime.database,
        "Surreal backend requested"
    );

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
    initializer.initialize_async().await?;

    let store_trait: Arc<dyn NodeStore> = store;
    let validator: Arc<dyn NodeValidator> = Arc::new(TreeSitterValidator);
    let embedding_provider = build_embedding_provider(Some(args))?;
    let avec_scorer = build_avec_scorer(Some(args));

    Ok(build_services(
        store_trait,
        validator,
        embedding_provider,
        avec_scorer,
    ))
}

fn build_avec_scorer(args: Option<&GatewayArgs>) -> Option<Arc<dyn AvecScorer>> {
    let args = args?;
    if !args.avec_scoring_enabled {
        return None;
    }

    Some(Arc::new(OllamaAvecScorer::new(
        args.avec_scoring_endpoint.clone(),
        args.avec_scoring_model.clone(),
    )))
}

fn build_embedding_provider(
    args: Option<&GatewayArgs>,
) -> Result<Option<Arc<dyn EmbeddingProvider>>> {
    let Some(args) = args else {
        return Ok(None);
    };

    if !args.embeddings_enabled {
        return Ok(None);
    }

    let provider: Arc<dyn EmbeddingProvider> = match args.embeddings_provider {
        EmbeddingsProviderKind::Ollama => Arc::new(OllamaEmbeddingProvider::new(
            args.embeddings_endpoint.clone(),
            args.embeddings_model.clone(),
        )),
        #[cfg(feature = "local-embedding")]
        EmbeddingsProviderKind::Local => Arc::new(LocalEmbeddingProvider::new(
            args.embeddings_model.clone(),
            args.embeddings_repo.clone(),
        )?),
    };

    Ok(Some(provider))
}
