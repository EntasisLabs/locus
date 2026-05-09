use locus_core_rs::parsing::SttpNodeParser;

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

#[test]
fn should_reject_prod_json_wrapper_payload_under_strict_typed_ir() {
    let parser = SttpNodeParser::new();
    let payload = r#"{"node_id":"sttp:runtime-phase-priority-checkpoint:2026-05-07","timestamp_utc":"2026-05-07T00:00:00Z","tier":"raw","context_summary":"Stasis runtime phase-priority checkpoint after major implementation burst. Team agreed to split final work into priority phases and proceed in order: P0 runtime safety/correctness, P1 spec fidelity (cron/timezone + continuation), P2 forensics/replay operability, P3 operational readiness.","avec":{"autonomy":0.84,"friction":0.36,"logic":0.82,"stability":0.77},"content":{"implemented_now":["Durable in-memory and Surreal runtimes","Outbox model + store + publisher flow","Outbox retry policy with backoff","Dead-letter replay API","Runtime factory/composition","Backend parity integration tests"],"gap_assessment":{"p0":"Need stronger atomic/CAS lease path in Surreal and stronger lease-expiry contention coverage.","p1":"Recurring model uses interval_seconds instead of cron_expr/timezone; continuation E2E missing.","p2":"job_attempt persistence and replay lineage diagnostics still missing.","p3":"Clock/IdGenerator ports and runtime metrics/retention are not implemented."},"priority_order":["P0","P1","P2","P3"],"next_action":"Start P0 immediately by making Surreal lease acquisition atomic and adding lease contention/recovery tests."},"confidence":{"overall":0.9,"notes":"Assessment based on direct code/doc comparison and green test state."}}"#;

    let parsed = parser.try_parse_strict_typed_ir(payload, "prod-json-wrapper");

    assert!(!parsed.success);
    assert!(parsed.diagnostics.iter().any(|d| {
        d.code == "STTP_PARSE_LAYER_ERROR" || d.code.starts_with("missing_layer_")
    }));
}

#[test]
fn should_reject_prod_shorthand_layer_payload_under_strict_typed_ir() {
    let parser = SttpNodeParser::new();
    let payload = r#"tier: raw
ts: 2026-05-07T00:00:00Z
⊕⟨summary⟩ Runtime checkpoint captured with priority phases P0->P3.
⦿⟨state⟩ scope: stasis-runtime; baseline: green.
◈⟨decision⟩ Start P0 now with atomic Surreal lease acquisition and recovery tests.
⍉⟨analysis⟩ Top risk is lease race under concurrent workers."#;

    let parsed = parser.try_parse_strict_typed_ir(payload, "prod-shorthand");

    assert!(!parsed.success);
    assert!(parsed.diagnostics.iter().any(|d| {
        d.code == "STTP_STRICT_PROFILE_VIOLATION"
            || d.code == "STTP_STRICT_MISSING_REQUIRED_KEY"
            || d.code == "STTP_STRICT_MISSING_REQUIRED_OBJECT"
            || d.code == "STTP_CONTENT_SCHEMA_MISSING_OBJECT"
    }));
}
