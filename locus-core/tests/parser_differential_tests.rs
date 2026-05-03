use locus_core::domain::contracts::NodeValidator;
use locus_core::{ParseProfile, SttpNodeParser, TreeSitterValidator};

const CANONICAL_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "diff-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "canonical", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "diff-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { test(.99): "canonical" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

const MISSING_CONTENT_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "diff-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "missing-content", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "diff-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

const WRONG_ORDER_NODE: &str = r#"
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "diff-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "diff-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "wrong-order", relevant_tier: raw, retrieval_budget: 3 } } ⟩
◈⟨ { test(.99): "wrong-order" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

const INVALID_CONTENT_SCHEMA_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "diff-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "invalid-content-schema", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "diff-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { topic: "missing confidence signature" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

const VALID_UNQUOTED_CONTENT_VALUE_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "diff-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "valid-unquoted-content", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "diff-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { topic(.91): unquoted_value } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

const INVALID_NESTED_CONTENT_SCHEMA_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "diff-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "invalid-nested-content-schema", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "diff-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { outer(.90): { inner: "missing confidence in nested field" } } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

#[test]
fn canonical_node_should_agree_between_profiles() {
    let parser = SttpNodeParser::new();

    let strict = parser.try_parse_strict(CANONICAL_NODE, "diff-test");
    let tolerant = parser.try_parse_tolerant(CANONICAL_NODE, "diff-test");

    assert!(strict.success, "strict parse failed: {:?}", strict.error);
    assert!(strict.strict_valid);
    assert!(
        tolerant.success,
        "tolerant parse failed: {:?}",
        tolerant.error
    );
    assert!(tolerant.strict_valid);
    assert_eq!(strict.profile, ParseProfile::Strict);
    assert_eq!(tolerant.profile, ParseProfile::Tolerant);

    let strict_node = strict.node.expect("strict node should exist");
    let tolerant_node = tolerant.node.expect("tolerant node should exist");

    assert_eq!(strict_node.tier, tolerant_node.tier);
    assert_eq!(
        strict_node.compression_depth,
        tolerant_node.compression_depth
    );
    assert!((strict_node.user_avec.psi() - tolerant_node.user_avec.psi()).abs() < 0.0001);
    assert!((strict_node.model_avec.psi() - tolerant_node.model_avec.psi()).abs() < 0.0001);
    assert!((strict_node.psi - tolerant_node.psi).abs() < 0.0001);

    let strict_ast = strict
        .canonical_ast
        .expect("strict canonical ast should exist");
    let content_layer = strict_ast.content.expect("content layer should exist");
    assert!(content_layer.span.start < content_layer.span.end);
    assert!(content_layer.source.contains("test(.99)"));
}

#[test]
fn missing_content_should_diverge_at_expected_boundary() {
    let parser = SttpNodeParser::new();
    let validator = TreeSitterValidator::new();

    let strict = parser.try_parse_strict(MISSING_CONTENT_NODE, "diff-test");
    let tolerant = parser.try_parse_tolerant(MISSING_CONTENT_NODE, "diff-test");
    let validation = validator.validate(MISSING_CONTENT_NODE);

    assert!(!validation.is_valid);
    assert!(!strict.success);
    assert!(!strict.strict_valid);
    assert!(tolerant.success);
    assert!(!tolerant.strict_valid);
    assert!(
        tolerant
            .diagnostics
            .iter()
            .any(|d| d.code == "missing_layer_content")
    );
}

#[test]
fn wrong_order_should_diverge_at_expected_boundary() {
    let parser = SttpNodeParser::new();
    let validator = TreeSitterValidator::new();

    let strict = parser.try_parse_strict(WRONG_ORDER_NODE, "diff-test");
    let tolerant = parser.try_parse_tolerant(WRONG_ORDER_NODE, "diff-test");
    let validation = validator.validate(WRONG_ORDER_NODE);

    assert!(!validation.is_valid);
    assert!(!strict.success);
    assert!(!strict.strict_valid);
    assert!(tolerant.success);
    assert!(!tolerant.strict_valid);
    assert!(
        tolerant
            .diagnostics
            .iter()
            .any(|d| d.code == "non_strict_spine_recovered_tolerantly")
    );
}

#[test]
fn content_schema_signature_should_diverge_at_expected_boundary() {
    let parser = SttpNodeParser::new();

    let strict = parser.try_parse_strict(INVALID_CONTENT_SCHEMA_NODE, "diff-test");
    let tolerant = parser.try_parse_tolerant(INVALID_CONTENT_SCHEMA_NODE, "diff-test");

    assert!(!strict.success);
    assert!(!strict.strict_valid);
    assert!(
        strict
            .diagnostics
            .iter()
            .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY")
    );
    assert!(
        strict
            .diagnostics
            .iter()
            .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY" && d.span.is_some())
    );

    assert!(tolerant.success);
    assert!(!tolerant.strict_valid);
    assert!(
        tolerant
            .diagnostics
            .iter()
            .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY")
    );
    assert!(
        tolerant
            .diagnostics
            .iter()
            .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY" && d.span.is_some())
    );
}

#[test]
fn content_schema_should_accept_unquoted_value_when_signature_is_valid() {
    let parser = SttpNodeParser::new();

    let strict = parser.try_parse_strict(VALID_UNQUOTED_CONTENT_VALUE_NODE, "diff-test");
    let tolerant = parser.try_parse_tolerant(VALID_UNQUOTED_CONTENT_VALUE_NODE, "diff-test");

    assert!(strict.success, "strict parse failed: {:?}", strict.error);
    assert!(strict.strict_valid);
    assert!(
        tolerant.success,
        "tolerant parse failed: {:?}",
        tolerant.error
    );
    assert!(tolerant.strict_valid);
    assert!(
        !strict
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("STTP_CONTENT_SCHEMA_"))
    );
}

#[test]
fn nested_content_schema_violation_should_be_reported() {
    let parser = SttpNodeParser::new();

    let strict = parser.try_parse_strict(INVALID_NESTED_CONTENT_SCHEMA_NODE, "diff-test");
    let tolerant = parser.try_parse_tolerant(INVALID_NESTED_CONTENT_SCHEMA_NODE, "diff-test");

    assert!(!strict.success);
    assert!(!strict.strict_valid);
    assert!(
        strict
            .diagnostics
            .iter()
            .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY")
    );

    assert!(tolerant.success);
    assert!(!tolerant.strict_valid);
    assert!(
        tolerant
            .diagnostics
            .iter()
            .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY")
    );
}
