use std::sync::Arc;

use chrono::{DateTime, Utc};
use locus_core_rs::application::services::{MonthlyRollupService, StoreContextService};
use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::models::MonthlyRollupRequest;
use locus_core_rs::parsing::SttpNodeParser;
use locus_core_rs::storage::InMemoryNodeStore;

#[tokio::test(flavor = "current_thread")]
async fn should_create_monthly_rollup_using_first_timeline_node_as_parent() {
    let store = Arc::new(InMemoryNodeStore::new());
    let validator = Arc::new(TreeSitterValidator);
    let store_context = StoreContextService::new(store.clone(), validator.clone());

    let first_store = store_context
        .store_async(
            &build_node(
                "first-session",
                "2026-03-05T10:20:00Z",
                0.91,
                0.21,
                0.86,
                0.94,
                0.88,
                0.90,
                2.92,
            ),
            "first-session",
        )
        .await;
    assert!(first_store.valid, "{:?}", first_store.validation_error);

    let second_store = store_context
        .store_async(
            &build_node(
                "second-session",
                "2026-03-21T20:43:43Z",
                0.82,
                0.28,
                0.87,
                0.63,
                0.91,
                0.88,
                2.60,
            ),
            "second-session",
        )
        .await;
    assert!(second_store.valid, "{:?}", second_store.validation_error);

    let service = MonthlyRollupService::new(store, validator);
    let result = service
        .create_async(MonthlyRollupRequest {
            session_id: "sttp-monthly-rollup-2026-04-04".to_string(),
            start_utc: parse_utc("2026-03-01T00:00:00Z"),
            end_utc: parse_utc("2026-04-01T00:00:00Z"),
            source_session_id: None,
            parent_node_id: None,
            limit: 5000,
            persist: true,
        })
        .await;

    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.source_nodes, 2);
    assert_eq!(result.parent_reference.as_deref(), Some("first-session"));
    assert!(result.raw_node.contains("parent_node: ref:first-session"));
    assert!((result.user_average.stability - 0.865).abs() <= 0.001);
    assert_eq!(result.rho_bands.high, 2);
    assert!(!result.node_id.trim().is_empty());
}

#[test]
fn should_parse_monthly_fixture_with_strict_typed_ir_without_missing_prime_keys() {
    let parser = SttpNodeParser::new();
    let raw = build_node(
        "strict-prime-session",
        "2026-03-05T10:20:00Z",
        0.91,
        0.21,
        0.86,
        0.94,
        0.88,
        0.90,
        2.92,
    );

    let parsed = parser.try_parse_strict_typed_ir(&raw, "strict-prime-session");

    assert!(parsed.success, "{:?}", parsed.error);
    assert!(!parsed.diagnostics.iter().any(|d| {
        d.code == "STTP_STRICT_MISSING_REQUIRED_KEY"
            && (d.message.contains("provenance.prime.relevant_tier")
                || d.message.contains("provenance.prime.retrieval_budget"))
    }));
}

fn parse_utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
        .expect("timestamp must parse")
        .with_timezone(&Utc)
}

#[allow(clippy::too_many_arguments)]
fn build_node(
    session_id: &str,
    timestamp: &str,
    user_stability: f32,
    user_friction: f32,
    user_logic: f32,
    user_autonomy: f32,
    rho: f32,
    kappa: f32,
    psi: f32,
) -> String {
    let avec_psi = user_stability + user_friction + user_logic + user_autonomy;

    format!(
        "⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: \"{session_id}\", compression_depth: 1, parent_node: null, prime: {{ attractor_config: {{ stability: {user_stability}, friction: {user_friction}, logic: {user_logic}, autonomy: {user_autonomy} }}, context_summary: \"test node\", relevant_tier: raw, retrieval_budget: 3 }} }} ⟩\n\
         ⦿⟨ {{ timestamp: \"{timestamp}\", tier: raw, session_id: \"{session_id}\", user_avec: {{ stability: {user_stability}, friction: {user_friction}, logic: {user_logic}, autonomy: {user_autonomy}, psi: {avec_psi} }}, model_avec: {{ stability: {user_stability}, friction: {user_friction}, logic: {user_logic}, autonomy: {user_autonomy}, psi: {avec_psi} }} }} ⟩\n\
         ◈⟨ {{ test(.99): \"service test\" }} ⟩\n\
         ⍉⟨ {{ rho: {rho}, kappa: {kappa}, psi: {psi}, compression_avec: {{ stability: {user_stability}, friction: {user_friction}, logic: {user_logic}, autonomy: {user_autonomy}, psi: {avec_psi} }} }} ⟩"
    )
}
