use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

use crate::domain::contracts::{EmbeddingProvider, NodeStore};
use crate::domain::models::{NodeQuery, NodeUpsertStatus, SttpNode};

#[derive(Debug, Clone, Default)]
pub struct EmbeddingMigrationFilter {
    pub session_id: Option<String>,
    pub from_utc: Option<DateTime<Utc>>,
    pub to_utc: Option<DateTime<Utc>>,
    pub tiers: Option<Vec<String>>,
    pub has_embedding: Option<bool>,
    pub embedding_model: Option<String>,
    pub sync_keys: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingMigrationPreviewRequest {
    pub filter: EmbeddingMigrationFilter,
    pub sample_limit: usize,
    pub max_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct EmbeddingMigrationSample {
    pub sync_key: String,
    pub session_id: String,
    pub tier: String,
    pub has_embedding: bool,
    pub embedding_model: Option<String>,
    pub embedding_dimensions: Option<usize>,
    pub embedded_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub context_summary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EmbeddingMigrationPreviewResult {
    pub total_candidates: usize,
    pub sample: Vec<EmbeddingMigrationSample>,
    pub provider_available: bool,
    pub provider_model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingMigrationMode {
    MissingOnly,
    ReindexAll,
}

#[derive(Debug, Clone)]
pub struct EmbeddingMigrationRunRequest {
    pub filter: EmbeddingMigrationFilter,
    pub mode: EmbeddingMigrationMode,
    pub dry_run: bool,
    pub batch_size: usize,
    pub max_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct EmbeddingMigrationRunResult {
    pub scanned: usize,
    pub selected: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub duplicate: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub provider_model: Option<String>,
    pub failure_reasons: Vec<String>,
}

pub struct EmbeddingMigrationService {
    store: Arc<dyn NodeStore>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl EmbeddingMigrationService {
    pub fn new(
        store: Arc<dyn NodeStore>,
        embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Self {
        Self {
            store,
            embedding_provider,
        }
    }

    pub async fn preview_async(
        &self,
        request: EmbeddingMigrationPreviewRequest,
    ) -> Result<EmbeddingMigrationPreviewResult> {
        let max_nodes = request.max_nodes.clamp(1, 50_000);
        let sample_limit = request.sample_limit.clamp(1, 200);
        let candidates = self.fetch_candidates(&request.filter, max_nodes).await?;

        Ok(EmbeddingMigrationPreviewResult {
            total_candidates: candidates.len(),
            sample: candidates
                .into_iter()
                .take(sample_limit)
                .map(to_sample)
                .collect::<Vec<_>>(),
            provider_available: self.embedding_provider.is_some(),
            provider_model: self
                .embedding_provider
                .as_ref()
                .map(|provider| provider.model_name().to_string()),
        })
    }

    pub async fn run_async(
        &self,
        request: EmbeddingMigrationRunRequest,
    ) -> Result<EmbeddingMigrationRunResult> {
        let started_at = Utc::now();
        let max_nodes = request.max_nodes.clamp(1, 50_000);
        let batch_size = request.batch_size.clamp(1, 500);
        let mut candidates = self.fetch_candidates(&request.filter, max_nodes).await?;
        let scanned = candidates.len();

        if request.mode == EmbeddingMigrationMode::MissingOnly {
            candidates.retain(|node| !node_has_embedding(node));
        }

        let selected = candidates.len();

        if !request.dry_run && self.embedding_provider.is_none() {
            return Err(anyhow!(
                "Embedding provider is not configured. Enable embeddings before running migration."
            ));
        }

        let provider_model = self
            .embedding_provider
            .as_ref()
            .map(|provider| provider.model_name().to_string());

        let mut result = EmbeddingMigrationRunResult {
            scanned,
            selected,
            updated: 0,
            skipped: 0,
            failed: 0,
            duplicate: 0,
            started_at,
            completed_at: started_at,
            provider_model,
            failure_reasons: Vec::new(),
        };

        if request.dry_run {
            result.updated = selected;
            result.completed_at = Utc::now();
            return Ok(result);
        }

        let provider = match self.embedding_provider.as_ref() {
            Some(provider) => provider,
            None => {
                return Err(anyhow!(
                    "Embedding provider is not configured. Enable embeddings before running migration."
                ));
            }
        };

        for batch in candidates.chunks(batch_size) {
            for mut node in batch.iter().cloned() {
                let Some(embedding_input) =
                    build_embedding_input(node.context_summary.as_deref(), &node.session_id)
                else {
                    result.skipped += 1;
                    continue;
                };

                let embedding = match provider.embed_async(&embedding_input).await {
                    Ok(values) if !values.is_empty() => values,
                    Ok(_) => {
                        result.failed += 1;
                        push_failure_reason(
                            &mut result.failure_reasons,
                            format!(
                                "{}: embedding provider returned an empty vector",
                                node.sync_key
                            ),
                        );
                        continue;
                    }
                    Err(err) => {
                        result.failed += 1;
                        push_failure_reason(
                            &mut result.failure_reasons,
                            format!("{}: embedding failed: {err}", node.sync_key),
                        );
                        continue;
                    }
                };

                node.embedding_dimensions = Some(embedding.len());
                node.embedding_model = Some(provider.model_name().to_string());
                node.embedding = Some(embedding);
                node.embedded_at = Some(Utc::now());
                node.updated_at = Utc::now();

                match self.store.upsert_node_async(node).await {
                    Ok(upsert) => match upsert.status {
                        NodeUpsertStatus::Created | NodeUpsertStatus::Updated => {
                            result.updated += 1;
                        }
                        NodeUpsertStatus::Duplicate => {
                            result.duplicate += 1;
                        }
                        NodeUpsertStatus::Skipped => {
                            result.skipped += 1;
                        }
                    },
                    Err(err) => {
                        result.failed += 1;
                        push_failure_reason(
                            &mut result.failure_reasons,
                            format!("store upsert failed: {err}"),
                        );
                    }
                }
            }
        }

        result.completed_at = Utc::now();
        Ok(result)
    }

    async fn fetch_candidates(
        &self,
        filter: &EmbeddingMigrationFilter,
        max_nodes: usize,
    ) -> Result<Vec<SttpNode>> {
        let tiers = normalize_tiers(filter.tiers.as_deref());
        let model_filter = normalize_model_filter(filter.embedding_model.as_deref());
        let sync_key_filter = normalize_sync_keys(filter.sync_keys.as_deref());

        let nodes = self
            .store
            .query_nodes_async(NodeQuery {
                limit: max_nodes,
                session_id: filter.session_id.clone(),
                from_utc: filter.from_utc,
                to_utc: filter.to_utc,
                tiers,
            })
            .await?;

        let filtered = nodes
            .into_iter()
            .filter(|node| match filter.has_embedding {
                Some(expected) => node_has_embedding(node) == expected,
                None => true,
            })
            .filter(|node| match model_filter.as_deref() {
                Some(expected) => node
                    .embedding_model
                    .as_ref()
                    .map(|model| model.eq_ignore_ascii_case(expected))
                    .unwrap_or(false),
                None => true,
            })
            .filter(|node| match sync_key_filter.as_ref() {
                Some(sync_keys) => sync_keys.contains(&node.sync_key),
                None => true,
            })
            .collect::<Vec<_>>();

        Ok(filtered)
    }
}

fn to_sample(node: SttpNode) -> EmbeddingMigrationSample {
    let has_embedding = node_has_embedding(&node);

    EmbeddingMigrationSample {
        sync_key: node.sync_key,
        session_id: node.session_id,
        tier: node.tier,
        has_embedding,
        embedding_model: node.embedding_model,
        embedding_dimensions: node.embedding_dimensions,
        embedded_at: node.embedded_at,
        updated_at: node.updated_at,
        context_summary: node.context_summary,
    }
}

fn normalize_tiers(tiers: Option<&[String]>) -> Option<Vec<String>> {
    let normalized = tiers
        .unwrap_or(&[])
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_model_filter(value: Option<&str>) -> Option<String> {
    value
        .map(|model| model.trim().to_ascii_lowercase())
        .filter(|model| !model.is_empty())
}

fn normalize_sync_keys(values: Option<&[String]>) -> Option<HashSet<String>> {
    let normalized = values
        .unwrap_or(&[])
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<HashSet<_>>();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn node_has_embedding(node: &SttpNode) -> bool {
    node.embedding
        .as_ref()
        .map(|values| !values.is_empty())
        .unwrap_or(false)
}

fn build_embedding_input(context_summary: Option<&str>, session_id: &str) -> Option<String> {
    let summary = context_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let session = session_id.trim();

    if summary.is_none() && session.is_empty() {
        return None;
    }

    Some(match summary {
        Some(summary) if !session.is_empty() => format!("{summary}\nsession_id:{session}"),
        Some(summary) => summary,
        None => format!("session_id:{session}"),
    })
}

fn push_failure_reason(reasons: &mut Vec<String>, reason: String) {
    if reasons.len() < 100 {
        reasons.push(reason);
    }
}
