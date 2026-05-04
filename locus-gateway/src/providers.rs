use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use locus_core::domain::contracts::EmbeddingProvider;
use locus_core::domain::models as core_models;

#[derive(Debug, Deserialize)]
struct ParsedAvecScore {
    stability: f32,
    friction: f32,
    logic: f32,
    autonomy: f32,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaChatMessage<'a>>,
    stream: bool,
    format: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatMessageOwned>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatMessageOwned {
    content: String,
}

#[async_trait]
pub(crate) trait AvecScorer: Send + Sync {
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
    async fn score_async(&self, text: &str) -> Result<core_models::AvecState>;
}

#[derive(Clone)]
pub(crate) struct OllamaAvecScorer {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaAvecScorer {
    pub(crate) fn new(endpoint: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint,
            model,
        }
    }
}

#[async_trait]
impl AvecScorer for OllamaAvecScorer {
    fn provider_name(&self) -> &str {
        "ollama"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn score_async(&self, text: &str) -> Result<core_models::AvecState> {
        let prompt = "Return ONLY valid compact JSON with numeric fields in [0,1]: stability, friction, logic, autonomy.";
        let response = self
            .client
            .post(&self.endpoint)
            .json(&OllamaChatRequest {
                model: &self.model,
                messages: vec![
                    OllamaChatMessage {
                        role: "system",
                        content: prompt,
                    },
                    OllamaChatMessage {
                        role: "user",
                        content: text,
                    },
                ],
                stream: false,
                format: json!("json"),
            })
            .send()
            .await?
            .error_for_status()?;

        let body: OllamaChatResponse = response.json().await?;
        let content = body
            .message
            .map(|message| message.content)
            .ok_or_else(|| anyhow!("ollama scoring response missing message content"))?;

        parse_avec_state_from_text(&content)
    }
}

pub(crate) async fn resolve_query_embedding(
    embedding_provider: Option<&Arc<dyn EmbeddingProvider>>,
    query_text: Option<&str>,
    provided_embedding: Option<&[f32]>,
) -> Option<Vec<f32>> {
    if let Some(embedding) = provided_embedding.filter(|embedding| !embedding.is_empty()) {
        return Some(embedding.to_vec());
    }

    let text = match query_text.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) {
        Some(text) => text,
        None => return None,
    };

    let provider = embedding_provider?;
    provider.embed_async(text).await.ok()
}

pub(crate) fn parse_avec_state_from_text(content: &str) -> Result<core_models::AvecState> {
    let parsed: ParsedAvecScore = match serde_json::from_str(content) {
        Ok(value) => value,
        Err(_) => {
            let start = content
                .find('{')
                .ok_or_else(|| anyhow!("AVEC scorer did not return JSON"))?;
            let end = content
                .rfind('}')
                .ok_or_else(|| anyhow!("AVEC scorer returned malformed JSON"))?;
            let candidate = &content[start..=end];
            serde_json::from_str(candidate)
                .map_err(|err| anyhow!("failed to parse AVEC JSON payload: {err}"))?
        }
    };

    Ok(core_models::AvecState {
        stability: parsed.stability.clamp(0.0, 1.0),
        friction: parsed.friction.clamp(0.0, 1.0),
        logic: parsed.logic.clamp(0.0, 1.0),
        autonomy: parsed.autonomy.clamp(0.0, 1.0),
    })
}
