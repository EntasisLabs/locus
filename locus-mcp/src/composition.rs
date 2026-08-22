use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::domain::contracts::EmbeddingProvider;
use locus_core_rs::{ParseProfile, SurrealDbSettings};
#[cfg(feature = "local-embedding")]
use locus_sdk::infrastructure::embeddings::LocalEmbeddingProvider;
use locus_sdk::infrastructure::embeddings::OllamaEmbeddingProvider;
use tracing::info;

pub(crate) use locus_surreal_adapter::RuntimeSurrealDbClient;

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

pub(crate) fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}

pub(crate) fn load_surreal_settings(args: &[String]) -> Result<SurrealDbSettings> {
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

pub(crate) fn runtime_args(args: &[String]) -> Vec<String> {
    let mut runtime_args = args.to_vec();
    if env_flag("LOCUS_MCP_REMOTE") && !runtime_args.iter().any(|value| value == "--remote") {
        runtime_args.push("--remote".to_string());
    }
    runtime_args
}

pub(crate) fn build_embedding_provider(args: &[String]) -> Result<Option<Arc<dyn EmbeddingProvider>>> {
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

pub(crate) fn resolve_parser_profile(args: &[String]) -> Result<ParseProfile> {
    let raw = env_or_arg("LOCUS_MCP_PARSE_PROFILE", args, "--parse-profile")
        .unwrap_or_else(|| "strict_typed_ir".to_string());

    parse_profile(raw.as_str()).ok_or_else(|| {
        anyhow::anyhow!(
            "unsupported parse profile '{}'; expected one of: strict_typed_ir, strict, tolerant",
            raw
        )
    })
}

fn parse_profile(value: &str) -> Option<ParseProfile> {
    match value.trim().to_ascii_lowercase().as_str() {
        "strict_typed_ir" | "strict-typed-ir" | "stricttypedir" | "typed_ir" | "typed-ir" => {
            Some(ParseProfile::StrictTypedIr)
        }
        "strict" => Some(ParseProfile::Strict),
        "tolerant" | "default" => Some(ParseProfile::Tolerant),
        _ => None,
    }
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

fn env_flag(key: &str) -> bool {
    std::env::var(key)
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            normalized == "1" || normalized == "true" || normalized == "yes"
        })
        .unwrap_or(false)
}
