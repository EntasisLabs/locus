use std::sync::Arc;

use anyhow::Result;
use serde_json::{Map, Value};
use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::NodeValidator;
use locus_core_rs::domain::models::AvecState;
use locus_core_rs::parsing::SttpNodeParser;
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

    let raw_node = render_sttp_node("sdk-composite-example", &result.content);

    let validator = TreeSitterValidator::new();
    let validation = validator.validate(&raw_node);
    println!("validator_valid={}", validation.is_valid);
    if let Some(err) = validation.error {
        println!("validator_error={err}");
    }

    let parser = SttpNodeParser::new();
    let parsed = parser.try_parse_strict(&raw_node, "sdk-composite-example");
    println!("strict_parse_success={}", parsed.success);
    if let Some(err) = parsed.error {
        println!("strict_parse_error={err}");
    }

    println!("\n--- sttp-node ---\n{raw_node}\n--- end ---");

    Ok(())
}

fn render_sttp_node(session_id: &str, content: &Value) -> String {
    let content_text = render_sttp_value(content);
    format!(
        "⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: \"{session_id}\", compression_depth: 1, parent_node: null, prime: {{ attractor_config: {{ stability: 0.80, friction: 0.20, logic: 0.85, autonomy: 0.75 }}, context_summary: \"sdk recursive composite example\", relevant_tier: raw, retrieval_budget: 5 }} }} ⟩\n\
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
