use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::domain::models::{
    AvecState, CanonicalAst, CanonicalAstLayer, ParseDiagnostic, ParseDiagnosticSeverity,
    ParseProfile, ParseResult, ParseSpan, SttpNode,
};
use crate::parsing::lexicon::{AVEC_COMPRESSION_KEY, AVEC_MODEL_KEY, AVEC_USER_KEY};
use crate::parsing::state_machine::{ParserState, SttpLayerStateMachine};

static TIMESTAMP_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"timestamp:\s*"(?P<v>[^"]+)""#).expect("timestamp regex must compile")
});

static TIER_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)tier:\s*(?P<v>raw|daily|weekly|monthly|quarterly|yearly)")
        .expect("tier regex must compile")
});

static COMPRESSION_DEPTH_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"compression_depth:\s*(?P<v>\d+)").expect("compression_depth regex must compile")
});

static PARENT_NODE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"parent_node:\s*(?:ref:(?P<ref>[^,\s}\]]+)|"(?P<quoted>[^"]+)"|(?P<null>null))"#)
        .expect("parent regex must compile")
});

static CONTEXT_SUMMARY_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"context_summary:\s*(?:"(?P<quoted>[^"]*)"|(?P<bare>[^,}\n]+))"#)
        .expect("context_summary regex must compile")
});

static AVEC_ENTRY_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?P<k>stability|friction|logic|autonomy|psi)\b\s*:\s*(?P<v>[-+]?\d*\.?\d+)")
        .expect("avec entry regex must compile")
});

static RHO_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"rho:\s*(?P<v>[-+]?\d*\.?\d+)").expect("rho regex must compile"));
static KAPPA_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"kappa:\s*(?P<v>[-+]?\d*\.?\d+)").expect("kappa regex must compile"));
static PSI_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"psi:\s*(?P<v>[-+]?\d*\.?\d+)").expect("psi regex must compile"));
static CONTENT_KEY_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<name>[A-Za-z_][A-Za-z0-9_]*)\(\s*(?P<c>[-+]?\d*\.?\d+)\s*\)$")
        .expect("content key regex must compile")
});

#[derive(Debug, Default, Clone, Copy)]
pub struct SttpNodeParser;

impl SttpNodeParser {
    pub fn new() -> Self {
        Self
    }

    pub fn try_parse(&self, raw: &str, session_id: &str) -> ParseResult {
        self.try_parse_with_profile(raw, session_id, ParseProfile::Tolerant)
    }

    pub fn try_parse_strict(&self, raw: &str, session_id: &str) -> ParseResult {
        self.try_parse_with_profile(raw, session_id, ParseProfile::Strict)
    }

    pub fn try_parse_tolerant(&self, raw: &str, session_id: &str) -> ParseResult {
        self.try_parse_with_profile(raw, session_id, ParseProfile::Tolerant)
    }

    pub fn try_parse_with_profile(
        &self,
        raw: &str,
        session_id: &str,
        profile: ParseProfile,
    ) -> ParseResult {
        let layered = SttpLayerStateMachine::parse(raw);
        let provenance = layered.provenance.unwrap_or(raw);
        let envelope = layered.envelope.unwrap_or(raw);
        let content = layered.content.unwrap_or(raw);
        let metrics = layered.metrics.unwrap_or(raw);
        let mut strict_valid = layered.strict_spine
            && layered.provenance.is_some()
            && layered.envelope.is_some()
            && layered.content.is_some()
            && layered.metrics.is_some();

        let mut diagnostics = to_structured_diagnostics(&layered.diagnostics);
        let canonical_ast = Some(CanonicalAst {
            provenance: layered
                .provenance
                .zip(layered.provenance_span)
                .map(|(source, span)| CanonicalAstLayer {
                    source: source.to_string(),
                    span: to_parse_span(span),
                }),
            envelope: layered
                .envelope
                .zip(layered.envelope_span)
                .map(|(source, span)| CanonicalAstLayer {
                    source: source.to_string(),
                    span: to_parse_span(span),
                }),
            content: layered
                .content
                .zip(layered.content_span)
                .map(|(source, span)| CanonicalAstLayer {
                    source: source.to_string(),
                    span: to_parse_span(span),
                }),
            metrics: layered
                .metrics
                .zip(layered.metrics_span)
                .map(|(source, span)| CanonicalAstLayer {
                    source: source.to_string(),
                    span: to_parse_span(span),
                }),
            strict_spine: layered.strict_spine,
            profile,
        });

        if matches!(layered.state, ParserState::Error) {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_PARSE_LAYER_ERROR".to_string(),
                message: "unable to identify any STTP layers".to_string(),
                severity: ParseDiagnosticSeverity::Fatal,
                strict_impact: true,
                span: None,
            });

            return ParseResult::fail_with_metadata(
                "unable to identify any STTP layers",
                profile,
                diagnostics,
                canonical_ast,
            );
        }

        if matches!(profile, ParseProfile::Strict) && !strict_valid {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_STRICT_PROFILE_VIOLATION".to_string(),
                message: "strict profile requires full layer spine provenance->envelope->content->metrics".to_string(),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: None,
            });

            return ParseResult::fail_with_metadata(
                "strict profile violation: missing or out-of-order layers",
                profile,
                diagnostics,
                canonical_ast,
            );
        }

        let content_diagnostics = validate_content_schema(raw, content, layered.content_span);
        if !content_diagnostics.is_empty() {
            strict_valid = false;
            for diag in content_diagnostics {
                diagnostics.push(diag);
            }

            if matches!(profile, ParseProfile::Strict) {
                return ParseResult::fail_with_metadata(
                    "strict profile violation: content schema requires field_name(.confidence): value",
                    profile,
                    diagnostics,
                    canonical_ast,
                );
            }
        }

        let user_avec = parse_avec_block(envelope, AVEC_USER_KEY)
            .or_else(|| parse_avec_block(raw, AVEC_USER_KEY))
            .unwrap_or_else(AvecState::zero);

        let model_avec = parse_avec_block(envelope, AVEC_MODEL_KEY)
            .or_else(|| parse_avec_block(raw, AVEC_MODEL_KEY))
            .unwrap_or_else(AvecState::zero);

        let compression_avec = parse_avec_block(metrics, AVEC_COMPRESSION_KEY)
            .or_else(|| parse_avec_block(raw, AVEC_COMPRESSION_KEY))
            .unwrap_or_else(AvecState::zero);

        let node = SttpNode {
            raw: raw.to_string(),
            session_id: session_id.to_string(),
            tier: parse_tier(envelope)
                .or_else(|| parse_tier(raw))
                .unwrap_or_default(),
            timestamp: parse_timestamp(envelope)
                .unwrap_or_else(|| parse_timestamp(raw).unwrap_or_else(Utc::now)),
            compression_depth: parse_int(&COMPRESSION_DEPTH_RX, provenance),
            parent_node_id: parse_parent_node(provenance).or_else(|| parse_parent_node(raw)),
            sync_key: String::new(),
            updated_at: Utc::now(),
            source_metadata: None,
            context_summary: parse_context_summary(provenance)
                .or_else(|| parse_context_summary(raw)),
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedded_at: None,
            user_avec,
            model_avec,
            compression_avec: Some(compression_avec),
            rho: parse_float(&RHO_RX, metrics),
            kappa: parse_float(&KAPPA_RX, metrics),
            psi: parse_float(&PSI_RX, metrics),
        };

        ParseResult::ok_with_metadata(node, profile, strict_valid, diagnostics, canonical_ast)
    }
}

fn validate_content_schema(
    raw_node: &str,
    content_layer: &str,
    layer_span: Option<crate::parsing::lexer::Span>,
) -> Vec<ParseDiagnostic> {
    let mut diagnostics = Vec::new();

    let Some(content_object) = extract_first_object(content_layer) else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_CONTENT_SCHEMA_MISSING_OBJECT".to_string(),
            message: "content layer must contain an object payload".to_string(),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: layer_span.map(to_parse_span),
        });
        return diagnostics;
    };

    let object_offset = offset_within(content_layer, content_object).unwrap_or(0);
    validate_object_schema(
        raw_node,
        content_layer,
        content_object,
        layer_span,
        object_offset,
        &mut diagnostics,
    );
    diagnostics
}

fn validate_object_schema(
    raw_node: &str,
    content_layer: &str,
    object_content: &str,
    layer_span: Option<crate::parsing::lexer::Span>,
    object_offset: usize,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    for pair in split_top_level_pairs(object_content) {
        let Some(colon_idx) = find_top_level_colon(pair.text) else {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_CONTENT_SCHEMA_INVALID_PAIR".to_string(),
                message: format!("content field missing ':' separator: {}", pair.text),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: project_content_span(
                    raw_node,
                    content_layer,
                    layer_span,
                    object_offset + pair.start,
                    pair.text.len(),
                ),
            });
            continue;
        };

        let raw_key = pair.text[..colon_idx].trim();
        let raw_value = pair.text[colon_idx + 1..].trim();

        let Some(caps) = CONTENT_KEY_RX.captures(raw_key) else {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_CONTENT_SCHEMA_INVALID_KEY".to_string(),
                message: format!(
                    "content key must match field_name(.confidence): found '{raw_key}'"
                ),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: project_content_span(
                    raw_node,
                    content_layer,
                    layer_span,
                    object_offset + pair.start,
                    raw_key.len(),
                ),
            });
            continue;
        };

        let confidence = caps
            .name("c")
            .and_then(|m| m.as_str().parse::<f32>().ok())
            .unwrap_or(-1.0);
        if !(0.0..=1.0).contains(&confidence) {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_CONTENT_SCHEMA_INVALID_CONFIDENCE".to_string(),
                message: format!(
                    "content confidence must be in [0,1]: found {confidence} for key '{raw_key}'"
                ),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: project_content_span(
                    raw_node,
                    content_layer,
                    layer_span,
                    object_offset + pair.start,
                    raw_key.len(),
                ),
            });
        }

        if raw_value.is_empty() {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_CONTENT_SCHEMA_MISSING_VALUE".to_string(),
                message: format!("content value is missing for key '{raw_key}'"),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: project_content_span(
                    raw_node,
                    content_layer,
                    layer_span,
                    object_offset + pair.start + colon_idx + 1,
                    1,
                ),
            });
            continue;
        }

        if raw_value.starts_with('{') && raw_value.ends_with('}') {
            if let Some(inner) = raw_value
                .strip_prefix('{')
                .and_then(|v| v.strip_suffix('}'))
            {
                let nested_offset = object_offset
                    + pair.start
                    + colon_idx
                    + 1
                    + pair.text[colon_idx + 1..].find('{').unwrap_or(0)
                    + 1;
                validate_object_schema(
                    raw_node,
                    content_layer,
                    inner,
                    layer_span,
                    nested_offset,
                    diagnostics,
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PairSlice<'a> {
    text: &'a str,
    start: usize,
}

fn split_top_level_pairs(input: &str) -> Vec<PairSlice<'_>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let mut in_quotes = false;
    let mut escape = false;

    for (idx, ch) in input.char_indices() {
        if in_quotes {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_quotes = false;
            }
            continue;
        }

        match ch {
            '"' => in_quotes = true,
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            ',' if depth_brace == 0 && depth_bracket == 0 => {
                let part = input[start..idx].trim();
                if !part.is_empty() {
                    let trimmed_start = start + input[start..idx].find(part).unwrap_or(0);
                    parts.push(PairSlice {
                        text: part,
                        start: trimmed_start,
                    });
                }
                start = idx + 1;
            }
            _ => {}
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        let trimmed_start = start + input[start..].find(tail).unwrap_or(0);
        parts.push(PairSlice {
            text: tail,
            start: trimmed_start,
        });
    }

    parts
}

fn find_top_level_colon(input: &str) -> Option<usize> {
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let mut in_quotes = false;
    let mut escape = false;

    for (idx, ch) in input.char_indices() {
        if in_quotes {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_quotes = false;
            }
            continue;
        }

        match ch {
            '"' => in_quotes = true,
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            ':' if depth_brace == 0 && depth_bracket == 0 => return Some(idx),
            _ => {}
        }
    }

    None
}

fn extract_first_object(input: &str) -> Option<&str> {
    let start = input.find('{')?;
    extract_braced_content(input, start)
}

fn to_parse_span(span: crate::parsing::lexer::Span) -> ParseSpan {
    ParseSpan {
        start: span.start,
        end: span.end,
        line: span.line,
        column: span.column,
    }
}

fn offset_within(haystack: &str, needle: &str) -> Option<usize> {
    let haystack_start = haystack.as_ptr() as usize;
    let needle_start = needle.as_ptr() as usize;
    let offset = needle_start.checked_sub(haystack_start)?;
    if offset <= haystack.len() {
        Some(offset)
    } else {
        None
    }
}

fn project_content_span(
    raw_node: &str,
    content_layer: &str,
    layer_span: Option<crate::parsing::lexer::Span>,
    local_offset_in_object: usize,
    len: usize,
) -> Option<ParseSpan> {
    let layer_span = layer_span?;
    let object_offset = extract_first_object(content_layer)
        .and_then(|obj| offset_within(content_layer, obj))
        .unwrap_or(0);

    let start = layer_span.start + object_offset + local_offset_in_object;
    let end = start.saturating_add(len.max(1));
    let (line, column) = line_col_at(raw_node, start);

    Some(ParseSpan {
        start,
        end,
        line,
        column,
    })
}

fn line_col_at(raw: &str, target_index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    let mut index = 0usize;

    for ch in raw.chars() {
        if index >= target_index {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }

        index += ch.len_utf8();
    }

    (line, column)
}

fn to_structured_diagnostics(codes: &[String]) -> Vec<ParseDiagnostic> {
    codes
        .iter()
        .map(|code| {
            let (message, severity, strict_impact) = match code.as_str() {
                "non_strict_spine_recovered_tolerantly" => (
                    "layer order deviates from strict spine; tolerant recovery applied",
                    ParseDiagnosticSeverity::Warning,
                    true,
                ),
                "missing_layer_provenance" => (
                    "provenance layer marker not found",
                    ParseDiagnosticSeverity::Error,
                    true,
                ),
                "missing_layer_envelope" => (
                    "envelope layer marker not found",
                    ParseDiagnosticSeverity::Error,
                    true,
                ),
                "missing_layer_content" => (
                    "content layer marker not found",
                    ParseDiagnosticSeverity::Warning,
                    true,
                ),
                "missing_layer_metrics" => (
                    "metrics layer marker not found",
                    ParseDiagnosticSeverity::Error,
                    true,
                ),
                _ => (
                    "parser emitted an unknown diagnostic",
                    ParseDiagnosticSeverity::Info,
                    false,
                ),
            };

            ParseDiagnostic {
                code: code.clone(),
                message: message.to_string(),
                severity,
                strict_impact,
                span: None,
            }
        })
        .collect()
}

fn parse_avec_block(source: &str, key: &str) -> Option<AvecState> {
    let object = extract_named_object(source, key)?;

    let mut stability = None;
    let mut friction = None;
    let mut logic = None;
    let mut autonomy = None;

    for caps in AVEC_ENTRY_RX.captures_iter(object) {
        let name = caps.name("k")?.as_str().to_ascii_lowercase();
        let value = caps
            .name("v")
            .and_then(|v| v.as_str().parse::<f32>().ok())
            .unwrap_or(0.0);

        match name.as_str() {
            "stability" => stability = Some(value),
            "friction" => friction = Some(value),
            "logic" => logic = Some(value),
            "autonomy" => autonomy = Some(value),
            _ => {}
        }
    }

    Some(AvecState {
        stability: stability?,
        friction: friction?,
        logic: logic?,
        autonomy: autonomy?,
    })
}

fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    let maybe_ts = TIMESTAMP_RX
        .captures(raw)
        .and_then(|c| c.name("v"))
        .map(|m| m.as_str());

    if let Some(ts) = maybe_ts {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(ts) {
            return Some(parsed.with_timezone(&Utc));
        }
    }

    None
}

fn parse_tier(raw: &str) -> Option<String> {
    TIER_RX
        .captures(raw)
        .and_then(|c| c.name("v"))
        .map(|m| m.as_str().to_string())
}

fn parse_parent_node(raw: &str) -> Option<String> {
    let caps = PARENT_NODE_RX.captures(raw)?;
    if caps.name("null").is_some() {
        return None;
    }

    if let Some(v) = caps.name("ref") {
        return Some(v.as_str().to_string());
    }

    if let Some(v) = caps.name("quoted") {
        return Some(v.as_str().to_string());
    }

    None
}

fn parse_context_summary(raw: &str) -> Option<String> {
    let caps = CONTEXT_SUMMARY_RX.captures(raw)?;
    let value = caps
        .name("quoted")
        .or_else(|| caps.name("bare"))
        .map(|m| m.as_str().trim())?;

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_int(rx: &Regex, raw: &str) -> i32 {
    rx.captures(raw)
        .and_then(|c| c.name("v"))
        .and_then(|v| v.as_str().parse::<i32>().ok())
        .unwrap_or(0)
}

fn parse_float(rx: &Regex, raw: &str) -> f32 {
    rx.captures(raw)
        .and_then(|c| c.name("v"))
        .and_then(|v| v.as_str().parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn extract_named_object<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let key_index = source.find(key)?;
    let after_key = &source[key_index + key.len()..];
    let colon_relative = after_key.find(':')?;
    let after_colon = &after_key[colon_relative + 1..];

    let brace_relative = after_colon.find('{')?;
    let absolute_brace_start = key_index + key.len() + colon_relative + 1 + brace_relative;
    extract_braced_content(source, absolute_brace_start)
}

fn extract_braced_content(source: &str, brace_start: usize) -> Option<&str> {
    let bytes = source.as_bytes();
    if *bytes.get(brace_start)? != b'{' {
        return None;
    }

    let mut depth = 0usize;
    for (idx, ch) in source[brace_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let content_start = brace_start + 1;
                    let content_end = brace_start + idx;
                    return source.get(content_start..content_end);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_avec_with_noncanonical_order() {
        let input =
            r#"user_avec: { logic: 0.90, stability: 0.81, autonomy: 0.92, friction: 0.11 }"#;
        let parsed = parse_avec_block(input, AVEC_USER_KEY).expect("avec should parse");

        assert!((parsed.stability - 0.81).abs() < 0.0001);
        assert!((parsed.friction - 0.11).abs() < 0.0001);
        assert!((parsed.logic - 0.90).abs() < 0.0001);
        assert!((parsed.autonomy - 0.92).abs() < 0.0001);
    }

    #[test]
    fn should_extract_nested_object_block() {
        let input = r#"compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, ext: { kept: 1 } }"#;
        let object = extract_named_object(input, AVEC_COMPRESSION_KEY).expect("block should parse");
        assert!(object.contains("stability"));
        assert!(object.contains("ext: { kept: 1 }"));
    }

    #[test]
    fn should_accept_content_value_with_or_without_quotes() {
        let quoted = r#"◈⟨ { topic(.91): \"quoted\" } ⟩"#;
        let unquoted = r#"◈⟨ { topic(.91): unquoted_value } ⟩"#;

        let quoted_diagnostics = validate_content_schema(quoted, quoted, None);
        let unquoted_diagnostics = validate_content_schema(unquoted, unquoted, None);

        assert!(quoted_diagnostics.is_empty());
        assert!(unquoted_diagnostics.is_empty());
    }

    #[test]
    fn should_reject_content_without_confidence_signature() {
        let content = r#"◈⟨ { topic: \"invalid\" } ⟩"#;
        let diagnostics = validate_content_schema(content, content, None);

        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_KEY")
        );
    }

    #[test]
    fn should_reject_content_confidence_out_of_range() {
        let content = r#"◈⟨ { topic(1.20): \"invalid\" } ⟩"#;
        let diagnostics = validate_content_schema(content, content, None);

        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "STTP_CONTENT_SCHEMA_INVALID_CONFIDENCE")
        );
    }

    #[test]
    fn should_extract_context_summary_from_prime() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "ctx-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, context_summary: "parser hardening session", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "ctx-test", user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
◈⟨ { note(.99): "ok" } ⟩
⍉⟨ { rho: 0.9, kappa: 0.9, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
"#;

        let parsed = parser.try_parse_tolerant(raw, "ctx-test");
        assert!(parsed.success);

        let node = parsed.node.expect("parsed node should exist");
        assert_eq!(
            node.context_summary.as_deref(),
            Some("parser hardening session")
        );
    }

    #[test]
    fn strict_profile_should_fail_on_missing_layer() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, compression_depth: 1 } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 } } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 } } ⟩
"#;

        let parsed = parser.try_parse_strict(raw, "strict-test");
        assert!(!parsed.success);
        assert_eq!(parsed.profile, ParseProfile::Strict);
        assert!(parsed.canonical_ast.is_some());
        assert!(!parsed.diagnostics.is_empty());
    }

    #[test]
    fn tolerant_profile_should_recover_with_diagnostics() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, compression_depth: 1 } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 } } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 } } ⟩
"#;

        let parsed = parser.try_parse_tolerant(raw, "tolerant-test");
        assert!(parsed.success);
        assert_eq!(parsed.profile, ParseProfile::Tolerant);
        assert!(!parsed.strict_valid);
        assert!(!parsed.diagnostics.is_empty());
        assert!(parsed.canonical_ast.is_some());
    }
}
