#[cfg(feature = "local-embedding")]
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use locus_core_rs::domain::contracts::EmbeddingProvider;
use serde::{Deserialize, Serialize};

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
pub struct OllamaEmbeddingProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaEmbeddingProvider {
    pub fn new(endpoint: String, model: String) -> Self {
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

#[cfg(feature = "local-embedding")]
pub struct LocalEmbeddingProvider {
    model_name: String,
    runtime: Arc<std::sync::Mutex<CandleRuntime>>,
}

#[cfg(feature = "local-embedding")]
impl LocalEmbeddingProvider {
    pub fn new(model_name: String, repo_id: String) -> Result<Self> {
        let runtime = CandleRuntime::new(&repo_id)?;

        Ok(Self {
            model_name: format!("local-{}", model_name.trim().to_lowercase()),
            runtime: Arc::new(std::sync::Mutex::new(runtime)),
        })
    }
}

#[cfg(feature = "local-embedding")]
#[async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    fn model_name(&self) -> &str {
        &self.model_name
    }

    async fn embed_async(&self, text: &str) -> Result<Vec<f32>> {
        use anyhow::Context;

        let runtime = Arc::clone(&self.runtime);
        let input = text.to_string();

        tokio::task::spawn_blocking(move || {
            let runtime = runtime
                .lock()
                .map_err(|_| anyhow!("Local embedding runtime lock poisoned"))?;
            runtime.embed(&input)
        })
        .await
        .context("embedding worker join failure")?
    }
}

#[cfg(feature = "local-embedding")]
struct CandleRuntime {
    model: candle_transformers::models::bert::BertModel,
    tokenizer: tokenizers::Tokenizer,
    device: candle_core::Device,
}

#[cfg(feature = "local-embedding")]
impl CandleRuntime {
    fn new(repo_id: &str) -> Result<Self> {
        use anyhow::Context;
        use candle_core::{DType, Device};
        use candle_nn::VarBuilder;
        use candle_transformers::models::bert::{BertModel, Config};
        use hf_hub::{Repo, RepoType, api::sync::ApiBuilder};
        use tokenizers::PaddingParams;

        let device = Device::Cpu;

        let api = ApiBuilder::new()
            .with_endpoint("https://huggingface.co".to_string())
            .build()
            .context("failed to create HuggingFace API client")?;
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

        let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
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
        use anyhow::Context;
        use candle_core::{DType, Tensor};

        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|err| anyhow!("tokenization failed: {err}"))?;

        let seq_len = encodings[0].get_ids().len();
        let batch_size = texts.len();

        let input_ids: Vec<u32> = encodings.iter().flat_map(|e| e.get_ids().to_vec()).collect();
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
            .context("local embedding forward pass failed")?;

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
