use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use locus_core_rs::domain::contracts::EmbeddingProvider;
use locus_sdk::domain::ai::{AiProviderRegistry, AiTask, ProviderPolicy};
use locus_sdk::infrastructure::registry::InMemoryAiProviderRegistry;
use locus_sdk::infrastructure::sttp_native::embedding_provider_adapter::SttpEmbeddingProviderAdapter;

#[cfg(feature = "genai-provider")]
use locus_sdk::infrastructure::genai_adapter::provider::GenaiProviderAdapter;

struct DemoEmbeddingProvider {
    model: String,
}

#[async_trait]
impl EmbeddingProvider for DemoEmbeddingProvider {
    fn model_name(&self) -> &str {
        &self.model
    }

    async fn embed_async(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.01, 0.02, 0.03])
    }
}

fn main() -> Result<()> {
    let mut registry = InMemoryAiProviderRegistry::new();

    let sttp_provider = Arc::new(DemoEmbeddingProvider {
        model: "demo-local".to_string(),
    });
    registry.register(SttpEmbeddingProviderAdapter::new("sttp-local", sttp_provider));

    #[cfg(feature = "genai-provider")]
    registry.register(GenaiProviderAdapter::new(
        "genai",
        Some("text-embedding-3-small".to_string()),
    ));

    let semantic = registry.resolve(
        AiTask::SemanticEmbedding,
        Some("sttp-local"),
        ProviderPolicy::Preferred,
    )?;
    println!("semantic provider: {}", semantic.provider_id());

    let any_avec = registry.resolve(AiTask::AvecEmbedding, None, ProviderPolicy::Auto)?;
    println!("avec embedding provider: {}", any_avec.provider_id());

    println!("registered capabilities:");
    for (id, caps) in registry.list_capabilities() {
        println!("- {id}: {:?}", caps);
    }

    Ok(())
}
