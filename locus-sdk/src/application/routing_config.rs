use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::ai::{AiTask, EmbedRequest, ScoreAvecRequest};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderModelProfile {
    pub semantic_model: Option<String>,
    pub avec_embedding_model: Option<String>,
    pub avec_scoring_model: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiRoutingConfig {
    pub default_provider_id: Option<String>,
    pub providers: HashMap<String, ProviderModelProfile>,
}

impl AiRoutingConfig {
    pub fn profile_for(&self, provider_id: &str) -> Option<&ProviderModelProfile> {
        self.providers.get(provider_id)
    }

    pub fn provider_for(&self, requested_provider_id: Option<&str>) -> Option<String> {
        requested_provider_id
            .filter(|value| !value.trim().is_empty())
            .map(ToString::to_string)
            .or_else(|| self.default_provider_id.clone())
    }

    pub fn model_for(&self, provider_id: Option<&str>, task: AiTask) -> Option<&str> {
        let provider_id = self.provider_for(provider_id)?;
        let profile = self.providers.get(&provider_id)?;

        match task {
            AiTask::SemanticEmbedding => profile.semantic_model.as_deref(),
            AiTask::AvecEmbedding => profile.avec_embedding_model.as_deref(),
            AiTask::AvecScoring => profile.avec_scoring_model.as_deref(),
        }
    }

    pub fn apply_to_embed_request(&self, request: &EmbedRequest) -> EmbedRequest {
        let provider_id = request
            .provider_id
            .as_deref()
            .map(ToString::to_string)
            .or_else(|| self.default_provider_id.clone());
        let model = request.model.clone().or_else(|| {
            self.model_for(provider_id.as_deref(), request.task)
                .map(ToString::to_string)
        });

        EmbedRequest {
            text: request.text.clone(),
            task: request.task,
            provider_id,
            model,
            policy: request.policy,
        }
    }

    pub fn apply_to_score_request(&self, request: &ScoreAvecRequest) -> ScoreAvecRequest {
        let provider_id = request
            .provider_id
            .as_deref()
            .map(ToString::to_string)
            .or_else(|| self.default_provider_id.clone());
        let model = request.model.clone().or_else(|| {
            self.model_for(provider_id.as_deref(), AiTask::AvecScoring)
                .map(ToString::to_string)
        });

        ScoreAvecRequest {
            text: request.text.clone(),
            provider_id,
            model,
            policy: request.policy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AiRoutingConfig, ProviderModelProfile};
    use crate::domain::ai::{AiTask, EmbedRequest, ProviderPolicy, ScoreAvecRequest};

    fn fixture_config() -> AiRoutingConfig {
        let mut config = AiRoutingConfig {
            default_provider_id: Some("genai".to_string()),
            providers: std::collections::HashMap::new(),
        };

        config.providers.insert(
            "genai".to_string(),
            ProviderModelProfile {
                semantic_model: Some("text-embedding-3-small".to_string()),
                avec_embedding_model: Some("text-embedding-3-large".to_string()),
                avec_scoring_model: Some("gpt-4o-mini".to_string()),
            },
        );

        config
    }

    #[test]
    fn apply_to_embed_request_fills_default_provider_and_model() {
        let config = fixture_config();
        let request = EmbedRequest {
            text: "hello".to_string(),
            task: AiTask::SemanticEmbedding,
            provider_id: None,
            model: None,
            policy: ProviderPolicy::Auto,
        };

        let resolved = config.apply_to_embed_request(&request);

        assert_eq!(resolved.provider_id.as_deref(), Some("genai"));
        assert_eq!(
            resolved.model.as_deref(),
            Some("text-embedding-3-small")
        );
    }

    #[test]
    fn apply_to_embed_request_keeps_explicit_model() {
        let config = fixture_config();
        let request = EmbedRequest {
            text: "hello".to_string(),
            task: AiTask::AvecEmbedding,
            provider_id: Some("genai".to_string()),
            model: Some("my-custom-model".to_string()),
            policy: ProviderPolicy::Preferred,
        };

        let resolved = config.apply_to_embed_request(&request);

        assert_eq!(resolved.model.as_deref(), Some("my-custom-model"));
    }

    #[test]
    fn apply_to_score_request_resolves_scoring_model() {
        let config = fixture_config();
        let request = ScoreAvecRequest {
            text: "score this".to_string(),
            provider_id: None,
            model: None,
            policy: ProviderPolicy::Auto,
        };

        let resolved = config.apply_to_score_request(&request);

        assert_eq!(resolved.provider_id.as_deref(), Some("genai"));
        assert_eq!(resolved.model.as_deref(), Some("gpt-4o-mini"));
    }
}
