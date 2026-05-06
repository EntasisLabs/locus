use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use locus_core_rs::domain::models::{AvecState, SttpNode};
use locus_core_rs::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    AiCapability, AiProvider, EmbedRequest, InMemoryAiProviderRegistry, MemoryCompositionService,
    MemoryDailyRollupRequest, MemoryRecallRequest, MemoryTransformOperation,
    MemoryTransformRequest, MemoryTransformThenRecallRequest, ScoreAvecRequest,
};

struct ExampleEmbeddingProvider;

#[async_trait]
impl AiProvider for ExampleEmbeddingProvider {
    fn provider_id(&self) -> &str {
        "example"
    }

    fn capabilities(&self) -> &'static [AiCapability] {
        &[AiCapability::SemanticEmbedding]
    }

    async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
        Ok(vec![0.1, 0.2, 0.3])
    }

    async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
        Ok(vec![0.1, 0.2, 0.3])
    }

    async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
        Ok(AvecState::zero())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    seed_demo_nodes(store.clone()).await?;

    let composition = MemoryCompositionService::new(store);

    let recall_with_explain = composition
        .recall_with_explain(&MemoryRecallRequest {
            query_text: Some("session".to_string()),
            ..Default::default()
        })
        .await?;

    println!(
        "recall_with_explain => retrieved={}, retrieval_path={:?}, stages={}",
        recall_with_explain.recall.retrieved,
        recall_with_explain.explain.retrieval_path,
        recall_with_explain.explain.stages.len()
    );

    let rollup = composition
        .daily_rollup(&MemoryDailyRollupRequest {
            max_days: 7,
            max_nodes: 100,
            ..Default::default()
        })
        .await?;

    println!(
        "daily_rollup => groups={}, scanned_nodes={}",
        rollup.total_groups, rollup.scanned_nodes
    );

    let schema = composition.capability_bundle();
    println!(
        "capability_bundle => schema_version={}, fallback_policies={}",
        schema.schema_version,
        schema.fallback_policies.join(",")
    );

    let mut providers = InMemoryAiProviderRegistry::new();
    providers.register(ExampleEmbeddingProvider);

    let transform_verify = composition
        .transform_then_recall_verify(
            Arc::new(providers),
            &MemoryTransformThenRecallRequest {
                transform: MemoryTransformRequest {
                    operation: MemoryTransformOperation::EmbedBackfill,
                    max_nodes: 100,
                    batch_size: 10,
                    ..Default::default()
                },
                recall: MemoryRecallRequest {
                    query_text: Some("session".to_string()),
                    ..Default::default()
                },
            },
        )
        .await?;

    println!(
        "transform_then_recall_verify => updated={}, retrieved={}",
        transform_verify.transform.updated, transform_verify.recall.retrieved
    );

    Ok(())
}

async fn seed_demo_nodes(store: Arc<dyn NodeStore>) -> Result<()> {
    let now = Utc::now();
    store.upsert_node_async(build_node("demo-a", now, "session memory alpha"))
        .await?;
    store.upsert_node_async(build_node("demo-b", now, "session memory beta"))
        .await?;
    Ok(())
}

fn build_node(session_id: &str, timestamp: chrono::DateTime<Utc>, raw: &str) -> SttpNode {
    let state = AvecState {
        stability: 0.55,
        friction: 0.35,
        logic: 0.75,
        autonomy: 0.65,
    };

    SttpNode {
        raw: raw.to_string(),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp,
        compression_depth: 1,
        parent_node_id: None,
        sync_key: format!("{}:{}", session_id, timestamp.timestamp_nanos_opt().unwrap_or_default()),
        updated_at: timestamp,
        source_metadata: None,
        context_summary: Some(raw.to_string()),
        embedding_dimensions: None,
        embedding_model: None,
        embedding: None,
        embedded_at: None,
        user_avec: state,
        model_avec: state,
        compression_avec: Some(state),
        rho: 0.9,
        kappa: 0.8,
        psi: 2.5,
    }
}
