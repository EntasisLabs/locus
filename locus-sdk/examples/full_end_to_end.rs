use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use locus_core::domain::models::{AvecState, SttpNode};
use locus_core::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    AiCapability, AiProvider, EmbedRequest, InMemoryAiProviderRegistry, MemoryAggregateRequest,
    MemoryAggregateService, MemoryExplainRequest, MemoryExplainService, MemoryFindRequest,
    MemoryFindService, MemoryGroupBy, MemoryRecallRequest, MemoryRecallService,
    MemorySchemaService, MemoryScoring, MemoryTransformOperation, MemoryTransformRequest,
    MemoryTransformService, ScoreAvecRequest,
};

struct ExampleProvider;

#[async_trait]
impl AiProvider for ExampleProvider {
    fn provider_id(&self) -> &str {
        "example-provider"
    }

    fn capabilities(&self) -> &'static [AiCapability] {
        &[
            AiCapability::SemanticEmbedding,
            AiCapability::AvecEmbedding,
            AiCapability::AvecScoring,
        ]
    }

    async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
        Ok(vec![0.11, 0.21, 0.31])
    }

    async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
        Ok(vec![0.13, 0.23, 0.33])
    }

    async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
        Ok(AvecState {
            stability: 0.72,
            friction: 0.28,
            logic: 0.84,
            autonomy: 0.66,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    seed_demo_nodes(store.clone()).await?;

    let find = MemoryFindService::new(store.clone());
    let recall = MemoryRecallService::new(store.clone());
    let explain = MemoryExplainService::new(store.clone());
    let aggregate = MemoryAggregateService::new(store.clone());
    let schema = MemorySchemaService::new();

    let mut providers = InMemoryAiProviderRegistry::new();
    providers.register(ExampleProvider);
    let providers: Arc<dyn locus_sdk::prelude::AiProviderRegistry> = Arc::new(providers);

    let transform = MemoryTransformService::new(store.clone(), providers.clone());

    let find_result = find
        .execute(&MemoryFindRequest {
            filter: locus_sdk::prelude::MemoryFilter {
                text_contains: Some("parser".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;
    println!(
        "find => retrieved={}, has_more={}",
        find_result.retrieved, find_result.has_more
    );

    let recall_request = MemoryRecallRequest {
        query_text: Some("parser".to_string()),
        scoring: MemoryScoring {
            fallback_policy: locus_sdk::prelude::FallbackPolicy::OnEmpty,
            ..Default::default()
        },
        ..Default::default()
    };

    let recall_result = recall.execute(&recall_request).await?;
    println!(
        "recall => retrieved={}, path={:?}, psi_avg={:.2}",
        recall_result.retrieved, recall_result.retrieval_path, recall_result.psi_range.average
    );

    let explain_result = explain
        .execute(&MemoryExplainRequest {
            recall: recall_request.clone(),
        })
        .await?;
    println!(
        "explain => stages={}, fallback_triggered={}",
        explain_result.stages.len(), explain_result.fallback_triggered
    );

    let aggregate_result = aggregate
        .execute(&MemoryAggregateRequest {
            group_by: MemoryGroupBy::DateDay,
            max_groups: 7,
            max_nodes: 100,
            ..Default::default()
        })
        .await?;
    println!(
        "aggregate => groups={}, scanned_nodes={}",
        aggregate_result.total_groups, aggregate_result.scanned_nodes
    );

    let dry_run_result = transform
        .execute(&MemoryTransformRequest {
            operation: MemoryTransformOperation::EmbedBackfill,
            dry_run: true,
            provider_id: Some("example-provider".to_string()),
            max_nodes: 100,
            batch_size: 25,
            ..Default::default()
        })
        .await?;
    println!(
        "transform dry-run => selected={}, updated={}, failed={}",
        dry_run_result.selected, dry_run_result.updated, dry_run_result.failed
    );

    let apply_result = transform
        .execute(&MemoryTransformRequest {
            operation: MemoryTransformOperation::EmbedBackfill,
            dry_run: false,
            provider_id: Some("example-provider".to_string()),
            max_nodes: 100,
            batch_size: 25,
            ..Default::default()
        })
        .await?;
    println!(
        "transform apply => updated={}, duplicate={}, failed={}",
        apply_result.updated, apply_result.duplicate, apply_result.failed
    );

    let schema_result = schema.execute();
    println!(
        "schema => version={}, operations={}",
        schema_result.schema_version,
        schema_result.transform_operations.join(",")
    );

    let composition = locus_sdk::prelude::MemoryCompositionService::new(store);
    let composition_result = composition.recall_with_explain(&recall_request).await?;
    println!(
        "composition recall_with_explain => recall={}, stages={}",
        composition_result.recall.retrieved,
        composition_result.explain.stages.len()
    );

    Ok(())
}
async fn seed_demo_nodes(store: Arc<dyn NodeStore>) -> Result<()> {
    let now = Utc::now();

    store
        .upsert_node_async(build_node(
            "e2e-session-a",
            now - Duration::minutes(12),
            "parser hardening and retrieval tuning",
            None,
        ))
        .await?;

    store
        .upsert_node_async(build_node(
            "e2e-session-a",
            now - Duration::minutes(7),
            "transport and gateway integration",
            Some(vec![0.2, 0.1, 0.4]),
        ))
        .await?;

    store
        .upsert_node_async(build_node(
            "e2e-session-b",
            now - Duration::minutes(3),
            "sdk composition and parser documentation",
            None,
        ))
        .await?;

    Ok(())
}

fn build_node(
    session_id: &str,
    timestamp: chrono::DateTime<Utc>,
    summary: &str,
    embedding: Option<Vec<f32>>,
) -> SttpNode {
    let user = AvecState {
        stability: 0.70,
        friction: 0.30,
        logic: 0.86,
        autonomy: 0.68,
    };
    let model = AvecState {
        stability: 0.66,
        friction: 0.24,
        logic: 0.90,
        autonomy: 0.64,
    };

    SttpNode {
        raw: format!("raw:{session_id}:{summary}"),
        session_id: session_id.to_string(),
        tier: "raw".to_string(),
        timestamp,
        compression_depth: 1,
        parent_node_id: None,
        sync_key: format!(
            "{}:{}",
            session_id,
            timestamp.timestamp_nanos_opt().unwrap_or_default()
        ),
        updated_at: timestamp,
        source_metadata: None,
        context_summary: Some(summary.to_string()),
        embedding_dimensions: embedding.as_ref().map(|v| v.len()),
        embedding_model: embedding.as_ref().map(|_| "seed-model".to_string()),
        embedding,
        embedded_at: None,
        user_avec: user,
        model_avec: model,
        compression_avec: Some(model),
        rho: 0.91,
        kappa: 0.86,
        psi: 2.54,
    }
}
