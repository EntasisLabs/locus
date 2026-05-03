use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use locus_core::domain::contracts::EmbeddingProvider;
use locus_core::domain::models::AvecState;

use crate::domain::ai::{AiCapability, AiProvider, EmbedRequest, ScoreAvecRequest};

pub struct SttpEmbeddingProviderAdapter {
    provider_id: String,
    embedding: Arc<dyn EmbeddingProvider>,
}

impl SttpEmbeddingProviderAdapter {
    pub fn new(provider_id: impl Into<String>, embedding: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            provider_id: provider_id.into(),
            embedding,
        }
    }
}

#[async_trait]
impl AiProvider for SttpEmbeddingProviderAdapter {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn capabilities(&self) -> &'static [AiCapability] {
        &[AiCapability::SemanticEmbedding, AiCapability::AvecEmbedding]
    }

    async fn embed_semantic(&self, request: &EmbedRequest) -> Result<Vec<f32>> {
        self.embedding.embed_async(&request.text).await
    }

    async fn embed_avec(&self, request: &EmbedRequest) -> Result<Vec<f32>> {
        self.embedding.embed_async(&request.text).await
    }

    async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
        Err(anyhow!("sttp embedding adapter does not implement AVEC scoring"))
    }
}
