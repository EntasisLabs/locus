use clap::{ArgAction, Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub(crate) struct GatewayArgs {
    #[arg(long, env = "STTP_GATEWAY_HTTP_PORT", default_value_t = 8080)]
    pub(crate) http_port: u16,

    #[arg(long, env = "STTP_GATEWAY_GRPC_PORT", default_value_t = 8081)]
    pub(crate) grpc_port: u16,

    #[arg(
        long,
        env = "STTP_GATEWAY_BACKEND",
        value_enum,
        default_value = "in-memory"
    )]
    pub(crate) backend: GatewayBackend,

    #[arg(
        long,
        env = "STTP_GATEWAY_ROOT_DIR_NAME",
        default_value = ".sttp-gateway"
    )]
    pub(crate) root_dir_name: String,

    #[arg(long, env = "STTP_GATEWAY_REMOTE", default_value_t = false)]
    pub(crate) remote: bool,

    #[arg(
        long,
        env = "STTP_GATEWAY_CORS_ENABLED",
        default_value_t = true,
        action = ArgAction::Set
    )]
    pub(crate) cors_enabled: bool,

    #[arg(long, env = "STTP_GATEWAY_CORS_ALLOWED_ORIGINS", default_value = "*")]
    pub(crate) cors_allowed_origins: String,

    #[arg(long, env = "STTP_SURREAL_EMBEDDED_ENDPOINT")]
    pub(crate) surreal_embedded_endpoint: Option<String>,

    #[arg(long, env = "STTP_SURREAL_REMOTE_ENDPOINT")]
    pub(crate) surreal_remote_endpoint: Option<String>,

    #[arg(long, env = "STTP_SURREAL_NAMESPACE", default_value = "keryx")]
    pub(crate) surreal_namespace: String,

    #[arg(long, env = "STTP_SURREAL_DATABASE", default_value = "sttp-mcp")]
    pub(crate) surreal_database: String,

    #[arg(long, env = "STTP_SURREAL_USER", default_value = "root")]
    pub(crate) surreal_user: String,

    #[arg(long, env = "STTP_SURREAL_PASSWORD", default_value = "root")]
    pub(crate) surreal_password: String,

    #[arg(long, env = "STTP_GATEWAY_EMBEDDINGS_ENABLED", default_value_t = false)]
    pub(crate) embeddings_enabled: bool,

    #[arg(
        long,
        env = "STTP_GATEWAY_EMBEDDINGS_PROVIDER",
        value_enum,
        default_value = "ollama"
    )]
    pub(crate) embeddings_provider: EmbeddingsProviderKind,

    #[arg(
        long,
        env = "STTP_GATEWAY_EMBEDDINGS_ENDPOINT",
        default_value = "http://127.0.0.1:11434/api/embeddings"
    )]
    pub(crate) embeddings_endpoint: String,

    #[arg(
        long,
        env = "STTP_GATEWAY_EMBEDDINGS_MODEL",
        default_value = "sttp-encoder"
    )]
    pub(crate) embeddings_model: String,

    #[arg(
        long,
        env = "STTP_GATEWAY_EMBEDDINGS_REPO",
        default_value = "sentence-transformers/all-MiniLM-L6-v2"
    )]
    pub(crate) embeddings_repo: String,

    #[arg(
        long,
        env = "STTP_GATEWAY_AVEC_SCORING_ENABLED",
        default_value_t = false
    )]
    pub(crate) avec_scoring_enabled: bool,

    #[arg(
        long,
        env = "STTP_GATEWAY_AVEC_SCORING_ENDPOINT",
        default_value = "http://127.0.0.1:11434/api/chat"
    )]
    pub(crate) avec_scoring_endpoint: String,

    #[arg(
        long,
        env = "STTP_GATEWAY_AVEC_SCORING_MODEL",
        default_value = "qwen2.5:0.5b"
    )]
    pub(crate) avec_scoring_model: String,
}

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum GatewayBackend {
    InMemory,
    Surreal,
}

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum EmbeddingsProviderKind {
    Ollama,
    #[cfg(feature = "candle-local")]
    Candle,
}
