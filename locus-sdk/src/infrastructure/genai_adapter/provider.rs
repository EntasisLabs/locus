use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use genai::Client;
use genai::chat::ChatRequest;
use locus_core_rs::domain::models::AvecState;

use crate::domain::ai::{AiCapability, AiProvider, EmbedRequest, ScoreAvecRequest};

pub struct GenaiProviderAdapter {
    provider_id: String,
    default_model: Option<String>,
    client: Client,
}

impl GenaiProviderAdapter {
    pub fn new(provider_id: impl Into<String>, default_model: Option<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            default_model,
            client: Client::default(),
        }
    }

    pub fn default_model(&self) -> Option<&str> {
        self.default_model.as_deref()
    }

    fn resolve_model<'a>(&'a self, requested: Option<&'a str>) -> Result<&'a str> {
        requested
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.default_model.as_deref())
            .ok_or_else(|| anyhow!("no model provided and no default model configured"))
    }
}

#[async_trait]
impl AiProvider for GenaiProviderAdapter {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn capabilities(&self) -> &'static [AiCapability] {
        &[
            AiCapability::SemanticEmbedding,
            AiCapability::AvecEmbedding,
            AiCapability::AvecScoring,
        ]
    }

    async fn embed_semantic(&self, request: &EmbedRequest) -> Result<Vec<f32>> {
        let model = self.resolve_model(request.model.as_deref())?;

        let response = self
            .client
            .embed(model, request.text.clone(), None)
            .await
            .with_context(|| format!("genai semantic embedding call failed for model '{model}'"))?;

        response
            .first_embedding()
            .map(|embedding| embedding.vector().to_vec())
            .filter(|vector| !vector.is_empty())
            .ok_or_else(|| anyhow!("genai semantic embedding response is missing vector data"))
    }

    async fn embed_avec(&self, request: &EmbedRequest) -> Result<Vec<f32>> {
        let model = self.resolve_model(request.model.as_deref())?;

        let response = self
            .client
            .embed(model, request.text.clone(), None)
            .await
            .with_context(|| format!("genai AVEC embedding call failed for model '{model}'"))?;

        response
            .first_embedding()
            .map(|embedding| embedding.vector().to_vec())
            .filter(|vector| !vector.is_empty())
            .ok_or_else(|| anyhow!("genai AVEC embedding response is missing vector data"))
    }

    async fn score_avec(&self, request: &ScoreAvecRequest) -> Result<AvecState> {
        let model = self.resolve_model(request.model.as_deref())?;
        let prompt = "Return only compact JSON with numeric fields in [0,1]: stability, friction, logic, autonomy.";

        let chat_req = ChatRequest::from_system(prompt).append_message(genai::chat::ChatMessage::user(
            request.text.clone(),
        ));

        let response = self
            .client
            .exec_chat(model, chat_req, None)
            .await
            .with_context(|| format!("genai AVEC scoring call failed for model '{model}'"))?;

        let content = response
            .first_text()
            .ok_or_else(|| anyhow!("genai AVEC scoring returned no text content"))?;

        parse_avec_state_from_text(content)
    }
}

#[derive(Debug, serde::Deserialize)]
struct ParsedAvecScore {
    stability: f32,
    friction: f32,
    logic: f32,
    autonomy: f32,
}

fn parse_avec_state_from_text(raw: &str) -> Result<AvecState> {
    let parsed: ParsedAvecScore = serde_json::from_str(raw)
        .with_context(|| "failed to parse AVEC JSON response from model")?;

    let values = [parsed.stability, parsed.friction, parsed.logic, parsed.autonomy];
    if values.iter().any(|value| !(0.0..=1.0).contains(value)) {
        return Err(anyhow!(
            "AVEC response contains values outside [0,1]: {:?}",
            values
        ));
    }

    Ok(AvecState {
        stability: parsed.stability,
        friction: parsed.friction,
        logic: parsed.logic,
        autonomy: parsed.autonomy,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_avec_state_from_text;

    #[test]
    fn parse_avec_state_accepts_valid_payload() {
        let raw = r#"{"stability":0.8,"friction":0.2,"logic":0.9,"autonomy":0.7}"#;
        let avec = parse_avec_state_from_text(raw).expect("expected valid AVEC payload");
        assert!((avec.stability - 0.8).abs() < f32::EPSILON);
        assert!((avec.friction - 0.2).abs() < f32::EPSILON);
        assert!((avec.logic - 0.9).abs() < f32::EPSILON);
        assert!((avec.autonomy - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_avec_state_rejects_out_of_range_values() {
        let raw = r#"{"stability":1.2,"friction":0.2,"logic":0.9,"autonomy":0.7}"#;
        let err = parse_avec_state_from_text(raw).expect_err("expected range validation error");
        assert!(err.to_string().contains("outside [0,1]"));
    }
}
