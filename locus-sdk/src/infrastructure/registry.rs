use std::collections::HashMap;

use anyhow::{Result, anyhow};

use crate::domain::ai::{AiCapability, AiProvider, AiProviderRegistry, AiTask, ProviderPolicy};

pub struct InMemoryAiProviderRegistry {
    providers: HashMap<String, Box<dyn AiProvider>>,
}

impl InMemoryAiProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register<P>(&mut self, provider: P)
    where
        P: AiProvider + 'static,
    {
        self.providers
            .insert(provider.provider_id().to_string(), Box::new(provider));
    }

    fn provider_supports_task(provider: &dyn AiProvider, task: AiTask) -> bool {
        let caps = provider.capabilities();
        match task {
            AiTask::SemanticEmbedding => caps.contains(&AiCapability::SemanticEmbedding),
            AiTask::AvecEmbedding => caps.contains(&AiCapability::AvecEmbedding),
            AiTask::AvecScoring => caps.contains(&AiCapability::AvecScoring),
        }
    }
}

impl Default for InMemoryAiProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AiProviderRegistry for InMemoryAiProviderRegistry {
    fn resolve(
        &self,
        task: AiTask,
        provider_id: Option<&str>,
        policy: ProviderPolicy,
    ) -> Result<&dyn AiProvider> {
        if let Some(id) = provider_id {
            let provider = self
                .providers
                .get(id)
                .ok_or_else(|| anyhow!("requested provider '{id}' is not registered"))?;
            if !Self::provider_supports_task(provider.as_ref(), task) {
                return Err(anyhow!("provider '{id}' does not support task '{task:?}'"));
            }
            return Ok(provider.as_ref());
        }

        match policy {
            ProviderPolicy::Required => Err(anyhow!(
                "provider_policy=required needs an explicit provider_id"
            )),
            ProviderPolicy::Auto | ProviderPolicy::Preferred => self
                .providers
                .values()
                .find(|provider| Self::provider_supports_task(provider.as_ref(), task))
                .map(|provider| provider.as_ref())
                .ok_or_else(|| anyhow!("no registered provider supports task '{task:?}'")),
        }
    }

    fn list_capabilities(&self) -> Vec<(String, Vec<AiCapability>)> {
        self.providers
            .iter()
            .map(|(id, provider)| (id.clone(), provider.capabilities().to_vec()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use async_trait::async_trait;
    use locus_core_rs::domain::models::AvecState;

    use super::InMemoryAiProviderRegistry;
    use crate::domain::ai::{
        AiCapability, AiProvider, AiProviderRegistry, AiTask, EmbedRequest, ProviderPolicy,
        ScoreAvecRequest,
    };

    struct SemanticOnlyProvider;

    #[async_trait]
    impl AiProvider for SemanticOnlyProvider {
        fn provider_id(&self) -> &str {
            "semantic-only"
        }

        fn capabilities(&self) -> &'static [AiCapability] {
            &[AiCapability::SemanticEmbedding]
        }

        async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![0.1, 0.2, 0.3])
        }

        async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![0.4, 0.5, 0.6])
        }

        async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
            Ok(AvecState {
                stability: 0.5,
                friction: 0.5,
                logic: 0.5,
                autonomy: 0.5,
            })
        }
    }

    struct AvecProvider;

    #[async_trait]
    impl AiProvider for AvecProvider {
        fn provider_id(&self) -> &str {
            "avec-provider"
        }

        fn capabilities(&self) -> &'static [AiCapability] {
            &[AiCapability::AvecScoring]
        }

        async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![1.0])
        }

        async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![1.0])
        }

        async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
            Ok(AvecState {
                stability: 0.7,
                friction: 0.2,
                logic: 0.9,
                autonomy: 0.6,
            })
        }
    }

    #[test]
    fn resolve_auto_picks_provider_supporting_task() {
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(SemanticOnlyProvider);

        let provider = registry
            .resolve(AiTask::SemanticEmbedding, None, ProviderPolicy::Auto)
            .expect("expected provider for semantic task");

        assert_eq!(provider.provider_id(), "semantic-only");
    }

    #[test]
    fn resolve_required_without_provider_id_fails() {
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(SemanticOnlyProvider);

        let err = match registry.resolve(AiTask::SemanticEmbedding, None, ProviderPolicy::Required)
        {
            Ok(_) => panic!("expected provider policy failure"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("provider_policy=required needs an explicit provider_id")
        );
    }

    #[test]
    fn resolve_with_explicit_provider_requires_capability_match() {
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(SemanticOnlyProvider);
        registry.register(AvecProvider);

        let err = match registry.resolve(
            AiTask::AvecScoring,
            Some("semantic-only"),
            ProviderPolicy::Preferred,
        ) {
            Ok(_) => panic!("expected capability mismatch"),
            Err(err) => err,
        };

        assert!(
            err.to_string()
                .contains("does not support task 'AvecScoring'")
        );
    }
}
