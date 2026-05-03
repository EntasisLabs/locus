use anyhow::Result;
use async_trait::async_trait;
use locus_core::domain::models::AvecState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCapability {
    SemanticEmbedding,
    AvecEmbedding,
    AvecScoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTask {
    SemanticEmbedding,
    AvecEmbedding,
    AvecScoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPolicy {
    Auto,
    Preferred,
    Required,
}

#[derive(Debug, Clone)]
pub struct EmbedRequest {
    pub text: String,
    pub task: AiTask,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub policy: ProviderPolicy,
}

#[derive(Debug, Clone)]
pub struct ScoreAvecRequest {
    pub text: String,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub policy: ProviderPolicy,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn capabilities(&self) -> &'static [AiCapability];

    async fn embed_semantic(&self, request: &EmbedRequest) -> Result<Vec<f32>>;

    async fn embed_avec(&self, request: &EmbedRequest) -> Result<Vec<f32>>;

    async fn score_avec(&self, request: &ScoreAvecRequest) -> Result<AvecState>;
}

pub trait AiProviderRegistry: Send + Sync {
    fn resolve(
        &self,
        task: AiTask,
        provider_id: Option<&str>,
        policy: ProviderPolicy,
    ) -> Result<&dyn AiProvider>;

    fn list_capabilities(&self) -> Vec<(String, Vec<AiCapability>)>;
}
