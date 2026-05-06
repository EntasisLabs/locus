use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::NodeValidator;

#[test]
fn should_validate_complete_node() {
    let validator = TreeSitterValidator;
    let node = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "regex-fix-test-2026-03-05", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "testing after regex patch", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "regex-fix-test-2026-03-05", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { test(.99): "regex patch for compression_avec parsing" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

    let result = validator.validate(node);
    assert!(
        result.is_valid,
        "validation failed: {:?} ({:?})",
        result.error, result.reason
    );
}

#[test]
fn should_reject_node_missing_layer() {
    let validator = TreeSitterValidator;
    let node = r#"
⊕⟨ { trigger: manual } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw } ⟩
◈⟨ { test: "value" } ⟩
"#;

    let result = validator.validate(node);
    assert!(!result.is_valid);
    assert!(
        result
            .error
            .unwrap_or_default()
            .contains("Missing required layer")
    );
}

#[test]
fn should_reject_node_with_wrong_layer_order() {
    let validator = TreeSitterValidator;
    let node = r#"
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw } ⟩
⊕⟨ { trigger: manual } ⟩
◈⟨ { test: "value" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60 } ⟩
"#;

    let result = validator.validate(node);
    assert!(!result.is_valid);
    assert!(
        result
            .error
            .unwrap_or_default()
            .contains("Layer order violation")
    );
}
