use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use locus_core::domain::contracts::EmbeddingProvider;
use locus_core::domain::models as core_models;

#[cfg(feature = "candle-local")]
use anyhow::Context;
#[cfg(feature = "candle-local")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "candle-local")]
use candle_nn::VarBuilder;
#[cfg(feature = "candle-local")]
use candle_transformers::models::bert::{BertModel, Config};
#[cfg(feature = "candle-local")]
use hf_hub::{Repo, RepoType, api::sync::Api};
#[cfg(feature = "candle-local")]
use std::sync::Mutex;
#[cfg(feature = "candle-local")]
use tokenizers::{PaddingParams, Tokenizer};

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Option<Vec<f32>>,
}

#[derive(Clone)]
pub(crate) struct OllamaEmbeddingProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaEmbeddingProvider {
    pub(crate) fn new(endpoint: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint,
            model,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbeddingProvider {
    fn model_name(&self) -> &str {
        &self.model
    }

    async fn embed_async(&self, text: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post(&self.endpoint)
            .json(&OllamaEmbeddingRequest {
                model: &self.model,
                prompt: text,
            })
            .send()
            .await?
            .error_for_status()?;

        let body: OllamaEmbeddingResponse = response.json().await?;
        match body.embedding {
            Some(embedding) if !embedding.is_empty() => Ok(embedding),
            _ => Err(anyhow!("embedding response missing vector")),
        }
    }
}

#[cfg(feature = "candle-local")]
pub(crate) struct SttpCandleProvider {
    model_name: String,
    runtime: Arc<Mutex<CandleRuntime>>,
}

#[cfg(feature = "candle-local")]
impl SttpCandleProvider {
    pub(crate) fn new(model_name: String, repo_id: String) -> Result<Self> {
        let runtime = CandleRuntime::new(&repo_id)?;

        Ok(Self {
            model_name: format!("candle-{}", model_name.trim().to_lowercase()),
            runtime: Arc::new(Mutex::new(runtime)),
        })
    }
}

#[cfg(feature = "candle-local")]
#[async_trait]
impl EmbeddingProvider for SttpCandleProvider {
    fn model_name(&self) -> &str {
        &self.model_name
    }

    async fn embed_async(&self, text: &str) -> Result<Vec<f32>> {
        let runtime = Arc::clone(&self.runtime);
        let input = text.to_string();

        tokio::task::spawn_blocking(move || {
            let runtime = runtime
                .lock()
                .map_err(|_| anyhow!("Candle runtime lock poisoned"))?;
            runtime.embed(&input)
        })
        .await
        .context("embedding worker join failure")?
    }
}

#[cfg(feature = "candle-local")]
struct CandleRuntime {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

#[cfg(feature = "candle-local")]
impl CandleRuntime {
    fn new(repo_id: &str) -> Result<Self> {
        let device = Device::Cpu;

        let api = Api::new().context("failed to create HuggingFace API client")?;
        let repo = api.repo(Repo::new(repo_id.to_string(), RepoType::Model));

        let config_path = repo
            .get("config.json")
            .with_context(|| format!("failed to fetch config.json from {repo_id}"))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .with_context(|| format!("failed to fetch tokenizer.json from {repo_id}"))?;
        let weights_path = repo
            .get("model.safetensors")
            .with_context(|| format!("failed to fetch model.safetensors from {repo_id}"))?;

        let config: Config = serde_json::from_str(
            &std::fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?,
        )
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

        let mut tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|err| anyhow!("tokenizer error: {err}"))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)
                .context("failed to map safetensors weights")?
        };
        let model = BertModel::load(vb, &config).context("failed to load BERT model")?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(&[text])?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("empty embedding output"))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|err| anyhow!("tokenization failed: {err}"))?;

        let seq_len = encodings[0].get_ids().len();
        let batch_size = texts.len();

        let input_ids: Vec<u32> = encodings
            .iter()
            .flat_map(|e| e.get_ids().to_vec())
            .collect();
        let attention_mask: Vec<u32> = encodings
            .iter()
            .flat_map(|e| e.get_attention_mask().to_vec())
            .collect();
        let token_type_ids: Vec<u32> = vec![0u32; batch_size * seq_len];

        let input_ids = Tensor::from_vec(input_ids, (batch_size, seq_len), &self.device)?;
        let attention_mask = Tensor::from_vec(attention_mask, (batch_size, seq_len), &self.device)?;
        let token_type_ids = Tensor::from_vec(token_type_ids, (batch_size, seq_len), &self.device)?;

        let output = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .context("Candle forward pass failed")?;

        let mask_f32 = attention_mask.to_dtype(DType::F32)?.unsqueeze(2)?;
        let masked = output.broadcast_mul(&mask_f32)?;
        let summed = masked.sum(1)?;
        let counts = mask_f32.sum(1)?;
        let pooled = summed.broadcast_div(&counts)?;

        let norm = pooled.sqr()?.sum_keepdim(1)?.sqrt()?;
        let normalized = pooled.broadcast_div(&norm)?;

        Ok(normalized.to_vec2::<f32>()?)
    }
}

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
