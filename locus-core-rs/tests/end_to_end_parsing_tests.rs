use std::collections::HashMap;

use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::NodeValidator;
use locus_core_rs::domain::models::AvecState;
use locus_core_rs::parsing::SttpNodeParser;

#[test]
fn should_parse_and_validate_complete_workflow() {
    let parser = SttpNodeParser::new();
    let validator = TreeSitterValidator;

    let node_text = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "regex-fix-test-2026-03-05", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "testing after regex patch", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "regex-fix-test-2026-03-05", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { test(.99): "regex patch for compression_avec parsing" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

    let validation = validator.validate(node_text);
    assert!(
        validation.is_valid,
        "validation failed: {:?}",
        validation.error
    );

    let parse_result = parser.try_parse(node_text, "regex-fix-test-2026-03-05");
    assert!(
        parse_result.success,
        "parse failed: {:?}",
        parse_result.error
    );

    let node = parse_result.node.expect("node must exist");

    assert!(node.user_avec.psi() > 0.0, "user_avec psi should be > 0");
    assert!(node.model_avec.psi() > 0.0, "model_avec psi should be > 0");
    let compression = node
        .compression_avec
        .expect("compression_avec should be present");
    assert!(
        compression.psi() > 0.0,
        "compression_avec psi should be > 0"
    );

    assert!((node.user_avec.stability - 0.85).abs() <= 0.0001);
    assert!((node.model_avec.stability - 0.85).abs() <= 0.0001);
    assert!((compression.stability - 0.85).abs() <= 0.0001);

    let user_avec_dict = create_avec_dict(node.user_avec);
    let model_avec_dict = create_avec_dict(node.model_avec);
    let compression_avec_dict = create_avec_dict(compression);

    assert!(!user_avec_dict.is_empty());
    assert!(!model_avec_dict.is_empty());
    assert!(!compression_avec_dict.is_empty());

    assert!((compression_avec_dict["stability"] - 0.85).abs() <= 0.0001);
    assert!((compression_avec_dict["friction"] - 0.25).abs() <= 0.0001);
    assert!((compression_avec_dict["logic"] - 0.80).abs() <= 0.0001);
    assert!((compression_avec_dict["autonomy"] - 0.70).abs() <= 0.0001);
    assert!(compression_avec_dict["psi"] > 0.0);
}

fn create_avec_dict(avec: AvecState) -> HashMap<String, f32> {
    HashMap::from([
        ("stability".to_string(), avec.stability),
        ("friction".to_string(), avec.friction),
        ("logic".to_string(), avec.logic),
        ("autonomy".to_string(), avec.autonomy),
        ("psi".to_string(), avec.psi()),
    ])
}
