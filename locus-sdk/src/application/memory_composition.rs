use std::sync::Arc;

use anyhow::Result;
use serde_json::{Map, Value};
use locus_core::domain::contracts::NodeStore;
use locus_core::domain::models::AvecState;

use crate::application::memory_aggregate::MemoryAggregateService;
use crate::application::memory_explain::MemoryExplainService;
use crate::application::manual_compression::ManualCompressionService;
use crate::application::memory_recall::MemoryRecallService;
use crate::application::memory_schema::MemorySchemaService;
use crate::application::memory_transform::MemoryTransformService;
use crate::domain::ai::AiProviderRegistry;
use crate::domain::compression::ManualCompressionRequest;
use crate::domain::memory::{
    MemoryAggregateRequest, MemoryAggregateResult, MemoryExplainRequest, MemoryExplainResult,
    MemoryFilter, MemoryGroupBy, MemoryRecallRequest, MemoryRecallResult, MemorySchemaResult,
    MemoryScope, MemoryTransformRequest, MemoryTransformResult, clamp_nodes,
};

#[derive(Debug, Clone)]
pub struct MemoryRecallWithExplainResult {
    pub recall: MemoryRecallResult,
    pub explain: MemoryExplainResult,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryDailyRollupRequest {
    pub scope: MemoryScope,
    pub filter: MemoryFilter,
    pub max_days: usize,
    pub max_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryTransformThenRecallRequest {
    pub transform: MemoryTransformRequest,
    pub recall: MemoryRecallRequest,
}

#[derive(Debug, Clone)]
pub struct MemoryTransformThenRecallResult {
    pub transform: MemoryTransformResult,
    pub recall: MemoryRecallResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeRole {
    User,
    Model,
    Document,
    Conversation,
}

impl CompositeRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Model => "model",
            Self::Document => "document",
            Self::Conversation => "conversation",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositeInputItem {
    pub role: CompositeRole,
    pub text: String,
    pub avec_override: Option<AvecState>,
    pub context: Vec<CompositeInputItem>,
}

#[derive(Debug, Clone, Default)]
pub struct CompositeRoleAvecOverrides {
    pub user: Option<AvecState>,
    pub model: Option<AvecState>,
    pub document: Option<AvecState>,
    pub conversation: Option<AvecState>,
}

impl CompositeRoleAvecOverrides {
    fn resolve(&self, role: CompositeRole) -> Option<AvecState> {
        match role {
            CompositeRole::User => self.user,
            CompositeRole::Model => self.model,
            CompositeRole::Document => self.document,
            CompositeRole::Conversation => self.conversation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositeNodeFromTextOptions {
    pub role_avec: CompositeRoleAvecOverrides,
    pub global_avec: Option<AvecState>,
    pub allow_llm_avec_fallback: bool,
    pub max_recursion_depth: usize,
}

impl Default for CompositeNodeFromTextOptions {
    fn default() -> Self {
        Self {
            role_avec: CompositeRoleAvecOverrides::default(),
            global_avec: None,
            allow_llm_avec_fallback: false,
            max_recursion_depth: 5,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompositeNodeFromTextRequest {
    pub items: Vec<CompositeInputItem>,
    pub options: CompositeNodeFromTextOptions,
}

#[derive(Debug, Clone, Default)]
pub struct CompositeNodeFromTextResult {
    pub content: Value,
    pub resolved_avec_count: usize,
    pub unresolved_avec_count: usize,
    pub requires_llm_avec: bool,
}

#[derive(Debug, Clone, Default)]
struct CompositeBuildStats {
    resolved_avec_count: usize,
    unresolved_avec_count: usize,
}

pub struct MemoryCompositionService {
    store: Arc<dyn NodeStore>,
    recall: MemoryRecallService,
    explain: MemoryExplainService,
    aggregate: MemoryAggregateService,
    schema: MemorySchemaService,
}

impl MemoryCompositionService {
    pub fn new(store: Arc<dyn NodeStore>) -> Self {
        Self {
            store: store.clone(),
            recall: MemoryRecallService::new(store.clone()),
            explain: MemoryExplainService::new(store.clone()),
            aggregate: MemoryAggregateService::new(store),
            schema: MemorySchemaService::new(),
        }
    }

    pub async fn recall_with_explain(
        &self,
        request: &MemoryRecallRequest,
    ) -> Result<MemoryRecallWithExplainResult> {
        let recall = self.recall.execute(request).await?;
        let explain = self
            .explain
            .execute(&MemoryExplainRequest {
                recall: request.clone(),
            })
            .await?;

        Ok(MemoryRecallWithExplainResult { recall, explain })
    }

    pub async fn daily_rollup(
        &self,
        request: &MemoryDailyRollupRequest,
    ) -> Result<MemoryAggregateResult> {
        let max_days = if request.max_days == 0 {
            30
        } else {
            request.max_days
        };
        let max_nodes = clamp_nodes(if request.max_nodes == 0 {
            5000
        } else {
            request.max_nodes
        });

        self.aggregate
            .execute(&MemoryAggregateRequest {
                scope: request.scope.clone(),
                filter: request.filter.clone(),
                group_by: MemoryGroupBy::DateDay,
                max_groups: max_days,
                max_nodes,
            })
            .await
    }

    pub fn capability_bundle(&self) -> MemorySchemaResult {
        self.schema.execute()
    }

    pub async fn transform_then_recall_verify(
        &self,
        providers: Arc<dyn AiProviderRegistry>,
        request: &MemoryTransformThenRecallRequest,
    ) -> Result<MemoryTransformThenRecallResult> {
        let transform_service = MemoryTransformService::new(self.store.clone(), providers);
        let transform = transform_service.execute(&request.transform).await?;
        let recall = self.recall.execute(&request.recall).await?;

        Ok(MemoryTransformThenRecallResult { transform, recall })
    }

    pub fn build_content_from_text(
        &self,
        request: &CompositeNodeFromTextRequest,
    ) -> Result<CompositeNodeFromTextResult> {
        let max_depth = request.options.max_recursion_depth.clamp(1, 5);
        let compressor = ManualCompressionService::new();
        let mut stats = CompositeBuildStats::default();

        let mut root = Map::new();
        for (idx, item) in request.items.iter().enumerate() {
            let key = format!("entry_{idx}(.95)");
            let value = build_composite_entry(
                item,
                1,
                max_depth,
                &request.options,
                &compressor,
                &mut stats,
            )?;
            root.insert(key, value);
        }

        let requires_llm = stats.unresolved_avec_count > 0;
        if requires_llm && !request.options.allow_llm_avec_fallback {
            anyhow::bail!(
                "unable to resolve AVEC for {} item(s); provide overrides or enable llm fallback",
                stats.unresolved_avec_count
            );
        }

        Ok(CompositeNodeFromTextResult {
            content: Value::Object(root),
            resolved_avec_count: stats.resolved_avec_count,
            unresolved_avec_count: stats.unresolved_avec_count,
            requires_llm_avec: requires_llm,
        })
    }
}

fn build_composite_entry(
    item: &CompositeInputItem,
    depth: usize,
    max_depth: usize,
    options: &CompositeNodeFromTextOptions,
    compressor: &ManualCompressionService,
    stats: &mut CompositeBuildStats,
) -> Result<Value> {
    if depth > max_depth {
        anyhow::bail!("composite context depth exceeded max depth of {max_depth}");
    }

    let resolved_avec = item
        .avec_override
        .or_else(|| options.role_avec.resolve(item.role))
        .or(options.global_avec);

    if resolved_avec.is_some() {
        stats.resolved_avec_count += 1;
    } else {
        stats.unresolved_avec_count += 1;
    }

    let compressed = compressor.execute(&ManualCompressionRequest {
        text: item.text.clone(),
        ..Default::default()
    });

    let mut entry = Map::new();
    entry.insert(
        "role(.99)".to_string(),
        Value::String(item.role.as_str().to_string()),
    );
    entry.insert("text(.70)".to_string(), Value::String(item.text.clone()));
    entry.insert(
        "anchor_topic(.86)".to_string(),
        Value::String(compressed.anchor_topic),
    );
    entry.insert(
        "key_points(.82)".to_string(),
        Value::Array(
            compressed
                .key_points
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );

    if let Some(avec) = resolved_avec {
        let mut avec_obj = Map::new();
        avec_obj.insert("stability(.99)".to_string(), Value::from(avec.stability as f64));
        avec_obj.insert("friction(.99)".to_string(), Value::from(avec.friction as f64));
        avec_obj.insert("logic(.99)".to_string(), Value::from(avec.logic as f64));
        avec_obj.insert("autonomy(.99)".to_string(), Value::from(avec.autonomy as f64));
        avec_obj.insert("psi(.99)".to_string(), Value::from(avec.psi() as f64));
        entry.insert("resolved_avec(.95)".to_string(), Value::Object(avec_obj));
    }

    if !item.context.is_empty() {
        let mut children = Map::new();
        for (idx, child) in item.context.iter().enumerate() {
            let child_key = format!("context_{idx}(.90)");
            children.insert(
                child_key,
                build_composite_entry(child, depth + 1, max_depth, options, compressor, stats)?,
            );
        }
        entry.insert("context(.88)".to_string(), Value::Object(children));
    }

    Ok(Value::Object(entry))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use serde_json::{Map, Value};
    use locus_core::application::validation::TreeSitterValidator;
    use locus_core::domain::contracts::NodeValidator;
    use locus_core::domain::models::{AvecState, SttpNode};
    use locus_core::parsing::SttpNodeParser;
    use locus_core::{InMemoryNodeStore, NodeStore};

    use super::{
        CompositeInputItem, CompositeNodeFromTextOptions, CompositeNodeFromTextRequest,
        CompositeRole, MemoryCompositionService, MemoryDailyRollupRequest,
        MemoryTransformThenRecallRequest,
    };
    use crate::domain::ai::{AiCapability, AiProvider, EmbedRequest, ScoreAvecRequest};
    use crate::domain::memory::{
        FallbackPolicy, MemoryFilter, MemoryRecallRequest, MemoryScoring, MemoryTransformOperation,
        MemoryTransformRequest, RetrievalPath,
    };
    use crate::infrastructure::registry::InMemoryAiProviderRegistry;

    struct MockEmbeddingProvider;

    #[async_trait]
    impl AiProvider for MockEmbeddingProvider {
        fn provider_id(&self) -> &str {
            "mock"
        }

        fn capabilities(&self) -> &'static [AiCapability] {
            &[AiCapability::SemanticEmbedding]
        }

        async fn embed_semantic(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![0.2, 0.3, 0.4])
        }

        async fn embed_avec(&self, _request: &EmbedRequest) -> Result<Vec<f32>> {
            Ok(vec![0.2, 0.3, 0.4])
        }

        async fn score_avec(&self, _request: &ScoreAvecRequest) -> Result<AvecState> {
            Ok(AvecState::zero())
        }
    }

    #[tokio::test]
    async fn recall_with_explain_returns_both_results() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        store
            .upsert_node_async(test_node("s-recipe", Utc::now(), "keyword in payload"))
            .await
            .expect("upsert should succeed");

        let service = MemoryCompositionService::new(store);
        let result = service
            .recall_with_explain(&MemoryRecallRequest {
                query_text: Some("keyword".to_string()),
                ..Default::default()
            })
            .await
            .expect("composed recall should succeed");

        assert!(!result.explain.stages.is_empty());
        assert!(result.recall.retrieved <= result.recall.nodes.len());
    }

    #[tokio::test]
    async fn recall_with_explain_marks_lexical_fallback_on_empty_policy() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        store
            .upsert_node_async(test_node("s-fallback", Utc::now(), "payload without match"))
            .await
            .expect("upsert should succeed");

        let service = MemoryCompositionService::new(store);
        let result = service
            .recall_with_explain(&MemoryRecallRequest {
                query_text: Some("needle".to_string()),
                filter: MemoryFilter {
                    has_embedding: Some(true),
                    ..Default::default()
                },
                scoring: MemoryScoring {
                    fallback_policy: FallbackPolicy::OnEmpty,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .expect("composed recall should succeed");

        assert_eq!(result.recall.retrieval_path, RetrievalPath::LexicalFallback);
        assert!(result.explain.fallback_triggered);
    }

    #[tokio::test]
    async fn daily_rollup_groups_by_day() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let now = Utc::now();
        store
            .upsert_node_async(test_node("s-rollup", now - Duration::days(1), "a"))
            .await
            .expect("upsert should succeed");
        store
            .upsert_node_async(test_node("s-rollup", now, "b"))
            .await
            .expect("upsert should succeed");

        let service = MemoryCompositionService::new(store);
        let result = service
            .daily_rollup(&MemoryDailyRollupRequest {
                max_days: 10,
                max_nodes: 100,
                ..Default::default()
            })
            .await
            .expect("daily rollup should succeed");

        assert!(result.total_groups >= 2);
    }

    #[test]
    fn capability_bundle_exposes_schema() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let service = MemoryCompositionService::new(store);
        let schema = service.capability_bundle();

        assert_eq!(schema.schema_version, "locus-sdk.memory.v1");
        assert!(schema
            .transform_operations
            .contains(&"embed_backfill".to_string()));
    }

    #[tokio::test]
    async fn transform_then_recall_verify_returns_both_sides() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        store
            .upsert_node_async(test_node("s-verify", Utc::now(), "verification payload"))
            .await
            .expect("upsert should succeed");

        let service = MemoryCompositionService::new(store.clone());

        let mut providers = InMemoryAiProviderRegistry::new();
        providers.register(MockEmbeddingProvider);

        let result = service
            .transform_then_recall_verify(
                Arc::new(providers),
                &MemoryTransformThenRecallRequest {
                    transform: MemoryTransformRequest {
                        operation: MemoryTransformOperation::EmbedBackfill,
                        dry_run: false,
                        max_nodes: 100,
                        batch_size: 10,
                        ..Default::default()
                    },
                    recall: MemoryRecallRequest {
                        query_text: Some("verification".to_string()),
                        ..Default::default()
                    },
                },
            )
            .await
            .expect("transform then recall should succeed");

        assert_eq!(result.transform.failed, 0);
        assert_eq!(result.transform.updated, 1);
        assert!(result.recall.retrieved <= result.recall.nodes.len());
    }

    #[test]
    fn build_content_from_text_resolves_avec_from_role_then_global() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let service = MemoryCompositionService::new(store);

        let role_state = AvecState {
            stability: 0.6,
            friction: 0.2,
            logic: 0.8,
            autonomy: 0.7,
        };
        let global_state = AvecState {
            stability: 0.4,
            friction: 0.3,
            logic: 0.6,
            autonomy: 0.5,
        };

        let result = service
            .build_content_from_text(&CompositeNodeFromTextRequest {
                items: vec![
                    CompositeInputItem {
                        role: CompositeRole::User,
                        text: "policy retrieval stability".to_string(),
                        avec_override: None,
                        context: Vec::new(),
                    },
                    CompositeInputItem {
                        role: CompositeRole::Document,
                        text: "technical writeup migration".to_string(),
                        avec_override: None,
                        context: Vec::new(),
                    },
                ],
                options: CompositeNodeFromTextOptions {
                    role_avec: super::CompositeRoleAvecOverrides {
                        user: Some(role_state),
                        ..Default::default()
                    },
                    global_avec: Some(global_state),
                    ..Default::default()
                },
            })
            .expect("composite build should succeed");

        assert_eq!(result.resolved_avec_count, 2);
        assert_eq!(result.unresolved_avec_count, 0);
        assert!(!result.requires_llm_avec);
    }

    #[test]
    fn build_content_from_text_fails_without_resolved_avec_when_llm_disabled() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let service = MemoryCompositionService::new(store);

        let err = service
            .build_content_from_text(&CompositeNodeFromTextRequest {
                items: vec![CompositeInputItem {
                    role: CompositeRole::Conversation,
                    text: "user asked then model replied".to_string(),
                    avec_override: None,
                    context: Vec::new(),
                }],
                options: CompositeNodeFromTextOptions {
                    allow_llm_avec_fallback: false,
                    ..Default::default()
                },
            })
            .expect_err("missing avec should fail when llm fallback is disabled");

        assert!(err.to_string().contains("unable to resolve AVEC"));
    }

    #[test]
    fn build_content_from_text_enforces_depth_limit() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let service = MemoryCompositionService::new(store);

        let leaf = CompositeInputItem {
            role: CompositeRole::User,
            text: "leaf".to_string(),
            avec_override: Some(AvecState::zero()),
            context: Vec::new(),
        };

        let depth2 = CompositeInputItem {
            role: CompositeRole::User,
            text: "depth2".to_string(),
            avec_override: Some(AvecState::zero()),
            context: vec![leaf],
        };

        let depth1 = CompositeInputItem {
            role: CompositeRole::User,
            text: "depth1".to_string(),
            avec_override: Some(AvecState::zero()),
            context: vec![depth2],
        };

        let err = service
            .build_content_from_text(&CompositeNodeFromTextRequest {
                items: vec![depth1],
                options: CompositeNodeFromTextOptions {
                    max_recursion_depth: 2,
                    ..Default::default()
                },
            })
            .expect_err("depth overflow should fail");

        assert!(err.to_string().contains("depth exceeded"));
    }

    #[test]
    fn composite_content_parses_and_validates_under_strict_profile() {
        let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
        let service = MemoryCompositionService::new(store);

        let request = CompositeNodeFromTextRequest {
            items: vec![CompositeInputItem {
                role: CompositeRole::Conversation,
                text: "user asked for deterministic recall and model proposed fallback policy".to_string(),
                avec_override: Some(AvecState {
                    stability: 0.8,
                    friction: 0.2,
                    logic: 0.85,
                    autonomy: 0.75,
                }),
                context: vec![CompositeInputItem {
                    role: CompositeRole::Document,
                    text: "system notes include lexical fallback and ranked retrieval".to_string(),
                    avec_override: Some(AvecState {
                        stability: 0.7,
                        friction: 0.3,
                        logic: 0.8,
                        autonomy: 0.7,
                    }),
                    context: Vec::new(),
                }],
            }],
            options: CompositeNodeFromTextOptions {
                allow_llm_avec_fallback: false,
                ..Default::default()
            },
        };

        let result = service
            .build_content_from_text(&request)
            .expect("composite content build should succeed");

        let raw_node = render_sttp_node("sdk-composite-session", &result.content);
        let validator = TreeSitterValidator::new();
        let validation = validator.validate(&raw_node);
        assert!(validation.is_valid, "validation failed: {:?}", validation.error);

        let parser = SttpNodeParser::new();
        let parse = parser.try_parse_strict(&raw_node, "sdk-composite-session");
        assert!(parse.success, "strict parse failed: {:?}", parse.error);
    }

    fn render_sttp_node(session_id: &str, content: &Value) -> String {
        let content_text = render_sttp_value(content);
        format!(
            "⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: \"{session_id}\", compression_depth: 1, parent_node: null, prime: {{ attractor_config: {{ stability: 0.80, friction: 0.20, logic: 0.85, autonomy: 0.75 }}, context_summary: \"sdk composite conformance\", relevant_tier: raw, retrieval_budget: 5 }} }} ⟩\n\
⦿⟨ {{ timestamp: \"2026-05-03T00:00:00Z\", tier: raw, session_id: \"{session_id}\", user_avec: {{ stability: 0.80, friction: 0.20, logic: 0.85, autonomy: 0.75, psi: 2.60 }}, model_avec: {{ stability: 0.82, friction: 0.18, logic: 0.84, autonomy: 0.74, psi: 2.58 }} }} ⟩\n\
◈⟨ {content_text} ⟩\n\
⍉⟨ {{ rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: {{ stability: 0.81, friction: 0.19, logic: 0.84, autonomy: 0.74, psi: 2.58 }} }} ⟩"
        )
    }

    fn render_sttp_value(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(v) => v.to_string(),
            Value::Number(v) => v.to_string(),
            Value::String(v) => format!("\"{}\"", v.replace('"', "\\\"")),
            Value::Array(values) => {
                let rendered = values
                    .iter()
                    .map(render_sttp_value)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{rendered}]")
            }
            Value::Object(obj) => render_sttp_object(obj),
        }
    }

    fn render_sttp_object(obj: &Map<String, Value>) -> String {
        let rendered = obj
            .iter()
            .map(|(key, value)| format!("{key}: {}", render_sttp_value(value)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{ {rendered} }}")
    }

    fn test_node(session_id: &str, timestamp: chrono::DateTime<Utc>, raw: &str) -> SttpNode {
        let state = AvecState {
            stability: 0.6,
            friction: 0.4,
            logic: 0.8,
            autonomy: 0.7,
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
}
