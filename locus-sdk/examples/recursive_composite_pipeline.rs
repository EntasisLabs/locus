use std::sync::Arc;

use anyhow::Result;
use chrono::{TimeZone, Utc};
use serde_json::Value;
use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::NodeValidator;
use locus_core_rs::domain::models::AvecState;
use locus_core_rs::parsing::{
    SttpContentSlice, SttpDocumentBuilder, SttpDocumentMetadata, SttpNodeParser,
};
use locus_core_rs::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    CompositeInputItem, CompositeNodeFromTextOptions, CompositeNodeFromTextRequest,
    CompositeRole, CompositeRoleAvecOverrides, MemoryCompositionService,
};

fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    let composition = MemoryCompositionService::new(store);

    let request = CompositeNodeFromTextRequest {
        items: vec![CompositeInputItem {
            role: CompositeRole::Conversation,
            text: "user asks for deterministic recall and model explains lexical fallback policy"
                .to_string(),
            avec_override: None,
            context: vec![
                CompositeInputItem {
                    role: CompositeRole::User,
                    text: "user is concerned about precision and auditability".to_string(),
                    avec_override: Some(AvecState {
                        stability: 0.82,
                        friction: 0.22,
                        logic: 0.88,
                        autonomy: 0.76,
                    }),
                    context: Vec::new(),
                },
                CompositeInputItem {
                    role: CompositeRole::Document,
                    text: "design notes mention strict parser compatibility and depth limit five"
                        .to_string(),
                    avec_override: None,
                    context: Vec::new(),
                },
            ],
        }],
        options: CompositeNodeFromTextOptions {
            role_avec: CompositeRoleAvecOverrides {
                conversation: Some(AvecState {
                    stability: 0.80,
                    friction: 0.20,
                    logic: 0.85,
                    autonomy: 0.75,
                }),
                document: Some(AvecState {
                    stability: 0.74,
                    friction: 0.24,
                    logic: 0.80,
                    autonomy: 0.72,
                }),
                ..Default::default()
            },
            global_avec: None,
            allow_llm_avec_fallback: false,
            max_recursion_depth: 5,
        },
    };

    let result = composition.build_content_from_text(&request)?;

    println!("resolved_avec_count={}", result.resolved_avec_count);
    println!("unresolved_avec_count={}", result.unresolved_avec_count);
    println!("requires_llm_avec={}", result.requires_llm_avec);

    let content = match result.content {
        Value::Object(map) => map,
        other => anyhow::bail!("expected object content, got {other}"),
    };
    let avec = AvecState {
        stability: 0.80,
        friction: 0.20,
        logic: 0.85,
        autonomy: 0.75,
    };
    let metadata = SttpDocumentMetadata::new("sdk-composite-example")
        .with_timestamp(
            Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0)
                .single()
                .expect("valid timestamp"),
        )
        .with_context_summary("sdk recursive composite example")
        .with_avec(avec, avec);

    let raw_node = SttpDocumentBuilder::new(metadata)
        .merge(SttpContentSlice::from_confidence_map(content)?)?
        .build()?
        .render_canonical();

    let validator = TreeSitterValidator::new();
    let validation = validator.validate(&raw_node);
    println!("validator_valid={}", validation.is_valid);
    if let Some(err) = validation.error {
        println!("validator_error={err}");
    }

    let parser = SttpNodeParser::new();
    let parsed = parser.try_parse_strict_typed_ir(&raw_node, "sdk-composite-example");
    println!("strict_typed_ir_success={}", parsed.success);
    if let Some(err) = parsed.error {
        println!("strict_typed_ir_error={err}");
    }

    println!("\n--- sttp-node ---\n{raw_node}\n--- end ---");

    Ok(())
}
