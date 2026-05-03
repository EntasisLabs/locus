use anyhow::Result;
use locus_core::domain::models::AvecState;

use crate::domain::ai::{AiProviderRegistry, EmbedRequest, ScoreAvecRequest};

pub async fn route_embedding(
    registry: &dyn AiProviderRegistry,
    request: &EmbedRequest,
) -> Result<Vec<f32>> {
    let provider = registry.resolve(request.task, request.provider_id.as_deref(), request.policy)?;

    match request.task {
        crate::domain::ai::AiTask::SemanticEmbedding => {
            provider.embed_semantic(request).await
        }
        crate::domain::ai::AiTask::AvecEmbedding => {
            provider.embed_avec(request).await
        }
        crate::domain::ai::AiTask::AvecScoring => {
            anyhow::bail!("AiTask::AvecScoring is not an embedding task")
        }
    }
}

pub async fn route_avec_score(
    registry: &dyn AiProviderRegistry,
    request: &ScoreAvecRequest,
) -> Result<AvecState> {
    let provider = registry.resolve(
        crate::domain::ai::AiTask::AvecScoring,
        request.provider_id.as_deref(),
        request.policy,
    )?;

    provider.score_avec(request).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::{Result, anyhow};
    use async_trait::async_trait;
    use locus_core::domain::models::AvecState;
    use tokio::sync::Mutex;

    use super::{route_avec_score, route_embedding};
    use crate::domain::ai::{
        AiCapability, AiProvider, AiProviderRegistry, AiTask, EmbedRequest, ProviderPolicy,
        ScoreAvecRequest,
    };
    use crate::infrastructure::registry::InMemoryAiProviderRegistry;

    #[derive(Clone, Default)]
    struct CallCounters {
        semantic_calls: Arc<Mutex<usize>>,
        avec_embedding_calls: Arc<Mutex<usize>>,
        avec_scoring_calls: Arc<Mutex<usize>>,
    }

    struct FullMockProvider {
        id: String,
        counters: CallCounters,
    }

    impl FullMockProvider {
        fn new(id: impl Into<String>, counters: CallCounters) -> Self {
            Self {
                id: id.into(),
                counters,
            }
        }
    }

    #[async_trait]
    impl AiProvider for FullMockProvider {
        fn provider_id(&self) -> &str {
            &self.id
        }

        fn capabilities(&self) -> &'static [AiCapability] {
            &[
                AiCapability::SemanticEmbedding,
                AiCapability::AvecEmbedding,
                AiCapability::AvecScoring,
            ]
        }

        async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            let mut calls = self.counters.semantic_calls.lock().await;
            *calls += 1;
            Ok(vec![0.1, 0.2, 0.3])
        }

        async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            let mut calls = self.counters.avec_embedding_calls.lock().await;
            *calls += 1;
            Ok(vec![0.4, 0.5, 0.6])
        }

        async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
            let mut calls = self.counters.avec_scoring_calls.lock().await;
            *calls += 1;
            Ok(AvecState {
                stability: 0.8,
                friction: 0.2,
                logic: 0.9,
                autonomy: 0.7,
            })
        }
    }

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
            Ok(vec![1.0])
        }

        async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Err(anyhow!("unsupported"))
        }

        async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
            Err(anyhow!("unsupported"))
        }
    }

    #[tokio::test]
    async fn route_embedding_dispatches_semantic_task() {
        let counters = CallCounters::default();
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(FullMockProvider::new("mock", counters.clone()));

        let request = EmbedRequest {
            text: "hello".to_string(),
            task: AiTask::SemanticEmbedding,
            provider_id: Some("mock".to_string()),
            model: None,
            policy: ProviderPolicy::Preferred,
        };

        let vector = route_embedding(&registry as &dyn AiProviderRegistry, &request)
            .await
            .expect("semantic routing should succeed");

        assert_eq!(vector, vec![0.1, 0.2, 0.3]);
        assert_eq!(*counters.semantic_calls.lock().await, 1);
        assert_eq!(*counters.avec_embedding_calls.lock().await, 0);
        assert_eq!(*counters.avec_scoring_calls.lock().await, 0);
    }

    #[tokio::test]
    async fn route_embedding_dispatches_avec_embedding_task() {
        let counters = CallCounters::default();
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(FullMockProvider::new("mock", counters.clone()));

        let request = EmbedRequest {
            text: "hello".to_string(),
            task: AiTask::AvecEmbedding,
            provider_id: Some("mock".to_string()),
            model: None,
            policy: ProviderPolicy::Preferred,
        };

        let vector = route_embedding(&registry as &dyn AiProviderRegistry, &request)
            .await
            .expect("AVEC embedding routing should succeed");

        assert_eq!(vector, vec![0.4, 0.5, 0.6]);
        assert_eq!(*counters.semantic_calls.lock().await, 0);
        assert_eq!(*counters.avec_embedding_calls.lock().await, 1);
        assert_eq!(*counters.avec_scoring_calls.lock().await, 0);
    }

    #[tokio::test]
    async fn route_avec_score_dispatches_scoring_task() {
        let counters = CallCounters::default();
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(FullMockProvider::new("mock", counters.clone()));

        let request = ScoreAvecRequest {
            text: "score this".to_string(),
            provider_id: Some("mock".to_string()),
            model: None,
            policy: ProviderPolicy::Preferred,
        };

        let avec = route_avec_score(&registry as &dyn AiProviderRegistry, &request)
            .await
            .expect("AVEC scoring should succeed");

        assert!((avec.stability - 0.8).abs() < f32::EPSILON);
        assert_eq!(*counters.semantic_calls.lock().await, 0);
        assert_eq!(*counters.avec_embedding_calls.lock().await, 0);
        assert_eq!(*counters.avec_scoring_calls.lock().await, 1);
    }

    #[tokio::test]
    async fn route_avec_score_fails_when_provider_lacks_capability() {
        let mut registry = InMemoryAiProviderRegistry::new();
        registry.register(SemanticOnlyProvider);

        let request = ScoreAvecRequest {
            text: "score this".to_string(),
            provider_id: Some("semantic-only".to_string()),
            model: None,
            policy: ProviderPolicy::Preferred,
        };

        let err = route_avec_score(&registry as &dyn AiProviderRegistry, &request)
            .await
            .expect_err("expected capability mismatch error");

        assert!(
            err.to_string()
                .contains("does not support task 'AvecScoring'")
        );
    }
}
