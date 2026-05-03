use locus_core::parsing::SttpNodeParser;

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn should_parse_valid_node_with_all_avec_blocks() {
    let parser = SttpNodeParser::new();
    let node = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "test-session", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "test node", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "test-session", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { test(.99): "unit test" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

    let result = parser.try_parse(node, "test-session");
    assert!(result.success, "parse failed: {:?}", result.error);

    let parsed = result.node.expect("parsed node must exist");

    assert_close(parsed.user_avec.stability, 0.85, 0.0001);
    assert_close(parsed.user_avec.friction, 0.25, 0.0001);
    assert_close(parsed.user_avec.logic, 0.80, 0.0001);
    assert_close(parsed.user_avec.autonomy, 0.70, 0.0001);
    assert_close(parsed.user_avec.psi(), 2.60, 0.01);

    assert_close(parsed.model_avec.stability, 0.85, 0.0001);
    assert_close(parsed.model_avec.friction, 0.25, 0.0001);
    assert_close(parsed.model_avec.logic, 0.80, 0.0001);
    assert_close(parsed.model_avec.autonomy, 0.70, 0.0001);
    assert_close(parsed.model_avec.psi(), 2.60, 0.01);

    let comp = parsed
        .compression_avec
        .expect("compression avec should be parsed");
    assert_close(comp.stability, 0.85, 0.0001);
    assert_close(comp.friction, 0.25, 0.0001);
    assert_close(comp.logic, 0.80, 0.0001);
    assert_close(comp.autonomy, 0.70, 0.0001);
    assert_close(comp.psi(), 2.60, 0.01);

    assert_eq!(parsed.tier, "raw");
    assert_eq!(parsed.compression_depth, 1);
    assert_close(parsed.rho, 0.96, 0.0001);
    assert_close(parsed.kappa, 0.94, 0.0001);
    assert_close(parsed.psi, 2.60, 0.01);
}

#[test]
fn should_parse_user_avec_block() {
    let parser = SttpNodeParser::new();
    let avec_block =
        "user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }";

    let input = format!(
        "⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: \"test\", compression_depth: 1, parent_node: null }} ⟩\n\
         ⦿⟨ {{ timestamp: \"2026-03-05T00:00:00Z\", tier: raw, session_id: \"test\", {avec_block}, model_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }} }} ⟩\n\
         ◈⟨ {{ test: \"value\" }} ⟩\n\
         ⍉⟨ {{ rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }} }} ⟩"
    );

    let result = parser.try_parse(&input, "test");
    assert!(result.success, "parse failed: {:?}", result.error);
    assert!(result.node.expect("node should exist").user_avec.psi() > 0.0);
}

#[test]
fn should_parse_model_avec_block() {
    let parser = SttpNodeParser::new();
    let avec_block =
        "model_avec: { stability: 0.86, friction: 0.24, logic: 0.93, autonomy: 0.84, psi: 2.87 }";

    let input = format!(
        "⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: \"test\", compression_depth: 1, parent_node: null }} ⟩\n\
         ⦿⟨ {{ timestamp: \"2026-03-05T00:00:00Z\", tier: raw, session_id: \"test\", user_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }}, {avec_block} }} ⟩\n\
         ◈⟨ {{ test: \"value\" }} ⟩\n\
         ⍉⟨ {{ rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }} }} ⟩"
    );

    let result = parser.try_parse(&input, "test");
    assert!(result.success, "parse failed: {:?}", result.error);

    let model = result.node.expect("node should exist").model_avec;
    assert_close(model.stability, 0.86, 0.0001);
    assert!(model.psi() > 0.0);
}

#[test]
fn should_parse_compression_avec_block() {
    let parser = SttpNodeParser::new();
    let avec_block = "compression_avec: { stability: 0.86, friction: 0.24, logic: 0.93, autonomy: 0.84, psi: 2.87 }";

    let input = format!(
        "⊕⟨ {{ trigger: manual, response_format: temporal_node, origin_session: \"test\", compression_depth: 1, parent_node: null }} ⟩\n\
         ⦿⟨ {{ timestamp: \"2026-03-05T00:00:00Z\", tier: raw, session_id: \"test\", user_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }}, model_avec: {{ stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }} }} ⟩\n\
         ◈⟨ {{ test: \"value\" }} ⟩\n\
         ⍉⟨ {{ rho: 0.96, kappa: 0.94, psi: 2.87, {avec_block} }} ⟩"
    );

    let result = parser.try_parse(&input, "test");
    assert!(result.success, "parse failed: {:?}", result.error);

    let comp = result
        .node
        .expect("node should exist")
        .compression_avec
        .expect("compression avec should exist");
    assert_close(comp.stability, 0.86, 0.0001);
    assert_close(comp.friction, 0.24, 0.0001);
    assert_close(comp.logic, 0.93, 0.0001);
    assert_close(comp.autonomy, 0.84, 0.0001);
    assert!(comp.psi() > 0.0);
}

#[test]
fn should_parse_generic_parent_reference() {
    let parser = SttpNodeParser::new();
    let input = "⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: \"test\", compression_depth: 1, parent_node: ref:parent-fix-check-2026-03-05 } ⟩\n\
                 ⦿⟨ { timestamp: \"2026-03-05T00:00:00Z\", tier: monthly, session_id: \"test\", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 } } ⟩\n\
                 ◈⟨ { test(.99): monthly_parent_ref } ⟩\n\
                 ⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 } } ⟩";

    let result = parser.try_parse(input, "test");
    assert!(result.success, "parse failed: {:?}", result.error);

    let parent = result
        .node
        .expect("node should exist")
        .parent_node_id
        .expect("parent id should parse");
    assert_eq!(parent, "parent-fix-check-2026-03-05");
}
