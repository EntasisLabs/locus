use super::ast::{AstLayer, SttpAst};
use super::lexer::{Token, TokenKind, tokenize};
use super::lexicon::{LAYER_ORDER, LAYER_STOP_MARKER, LayerKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    Start,
    InProvenance,
    InEnvelope,
    InContent,
    InMetrics,
    Done,
    Error,
}

#[derive(Debug, Clone)]
pub struct LayerParseOutcome<'a> {
    pub provenance: Option<&'a str>,
    pub provenance_span: Option<super::lexer::Span>,
    pub envelope: Option<&'a str>,
    pub envelope_span: Option<super::lexer::Span>,
    pub content: Option<&'a str>,
    pub content_span: Option<super::lexer::Span>,
    pub metrics: Option<&'a str>,
    pub metrics_span: Option<super::lexer::Span>,
    pub strict_spine: bool,
    pub state: ParserState,
    pub diagnostics: Vec<String>,
}

pub struct SttpLayerStateMachine;

impl SttpLayerStateMachine {
    pub fn parse<'a>(raw: &'a str) -> LayerParseOutcome<'a> {
        let mut diagnostics = Vec::new();
        let tokens = tokenize(raw);

        let provenance = extract_layer(raw, &tokens, LayerKind::Provenance);
        let envelope = extract_layer(raw, &tokens, LayerKind::Envelope);
        let content = extract_layer(raw, &tokens, LayerKind::Content);
        let metrics = extract_layer(raw, &tokens, LayerKind::Metrics);

        let provenance_span = provenance.as_ref().map(|v| v.span);
        let envelope_span = envelope.as_ref().map(|v| v.span);
        let content_span = content.as_ref().map(|v| v.span);
        let metrics_span = metrics.as_ref().map(|v| v.span);

        let provenance = provenance.as_ref().map(|v| v.slice);
        let envelope = envelope.as_ref().map(|v| v.slice);
        let content = content.as_ref().map(|v| v.slice);
        let metrics = metrics.as_ref().map(|v| v.slice);

        let strict_spine = is_strict_spine(raw);
        if !strict_spine {
            diagnostics.push("non_strict_spine_recovered_tolerantly".to_string());
        }

        if provenance.is_none() {
            diagnostics.push("missing_layer_provenance".to_string());
        }
        if envelope.is_none() {
            diagnostics.push("missing_layer_envelope".to_string());
        }
        if content.is_none() {
            diagnostics.push("missing_layer_content".to_string());
        }
        if metrics.is_none() {
            diagnostics.push("missing_layer_metrics".to_string());
        }

        let state = if metrics.is_some() {
            ParserState::Done
        } else if content.is_some() {
            ParserState::InContent
        } else if envelope.is_some() {
            ParserState::InEnvelope
        } else if provenance.is_some() {
            ParserState::InProvenance
        } else {
            ParserState::Error
        };

        LayerParseOutcome {
            provenance,
            provenance_span,
            envelope,
            envelope_span,
            content,
            content_span,
            metrics,
            metrics_span,
            strict_spine,
            state,
            diagnostics,
        }
    }

    pub fn to_ast(raw: &str, parsed: &LayerParseOutcome<'_>) -> SttpAst {
        let make_layer = |kind: LayerKind, source: Option<&str>| {
            source.and_then(|slice| {
                let start = raw.find(slice)?;
                let end = start + slice.len();
                Some(AstLayer {
                    kind,
                    source: slice.to_string(),
                    start,
                    end,
                })
            })
        };

        SttpAst {
            provenance: make_layer(LayerKind::Provenance, parsed.provenance),
            envelope: make_layer(LayerKind::Envelope, parsed.envelope),
            content: make_layer(LayerKind::Content, parsed.content),
            metrics: make_layer(LayerKind::Metrics, parsed.metrics),
            strict_spine: parsed.strict_spine,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LayerSlice<'a> {
    slice: &'a str,
    span: super::lexer::Span,
}

fn is_strict_spine(raw: &str) -> bool {
    let mut last = 0usize;
    for kind in LAYER_ORDER {
        let marker = kind.marker();
        let Some(relative) = raw[last..].find(marker) else {
            return false;
        };
        last += relative + marker.len();
    }

    true
}

fn extract_layer<'a>(raw: &'a str, tokens: &[Token], kind: LayerKind) -> Option<LayerSlice<'a>> {
    let layer_start_kind = layer_start_token_kind(kind);
    let start_token_index = tokens.iter().position(|t| t.kind == layer_start_kind)?;
    let start_token = tokens[start_token_index];
    let start = tokens[start_token_index].span.start;

    let end = tokens
        .iter()
        .skip(start_token_index + 1)
        .find(|t| t.kind == TokenKind::LayerEnd)
        .map(|t| t.span.end)
        .or_else(|| {
            let value_start = start + kind.marker().len();
            raw[value_start..]
                .find(LAYER_STOP_MARKER)
                .map(|relative| value_start + relative + LAYER_STOP_MARKER.len())
        })?;

    let slice = raw.get(start..end)?;
    Some(LayerSlice {
        slice,
        span: super::lexer::Span {
            start,
            end,
            line: start_token.span.line,
            column: start_token.span.column,
        },
    })
}

fn layer_start_token_kind(kind: LayerKind) -> TokenKind {
    match kind {
        LayerKind::Provenance => TokenKind::ProvenanceStart,
        LayerKind::Envelope => TokenKind::EnvelopeStart,
        LayerKind::Content => TokenKind::ContentStart,
        LayerKind::Metrics => TokenKind::MetricsStart,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_detect_strict_spine() {
        let raw = r#"
⊕⟨ { trigger: manual } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw } ⟩
◈⟨ { a(.99): b } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 0.3 } ⟩
"#;

        let parsed = SttpLayerStateMachine::parse(raw);
        assert!(parsed.strict_spine);
        assert_eq!(parsed.state, ParserState::Done);
        assert!(parsed.provenance.is_some());
        assert!(parsed.envelope.is_some());
        assert!(parsed.content.is_some());
        assert!(parsed.metrics.is_some());
    }

    #[test]
    fn should_recover_with_missing_layer() {
        let raw = r#"
⊕⟨ { trigger: manual } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 0.3 } ⟩
"#;

        let parsed = SttpLayerStateMachine::parse(raw);
        assert!(!parsed.strict_spine);
        assert_eq!(parsed.state, ParserState::Done);
        assert!(parsed.content.is_none());
        assert!(parsed.metrics.is_some());
        assert!(parsed.diagnostics.len() >= 2);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d == "missing_layer_content")
        );
    }

    #[test]
    fn should_project_ast_with_spans() {
        let raw = r#"
⊕⟨ { trigger: manual } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw } ⟩
◈⟨ { topic(.99): x } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 0.3 } ⟩
"#;

        let parsed = SttpLayerStateMachine::parse(raw);
        let ast = SttpLayerStateMachine::to_ast(raw, &parsed);

        assert!(ast.provenance.is_some());
        assert!(ast.envelope.is_some());
        assert!(ast.content.is_some());
        assert!(ast.metrics.is_some());

        let content = ast.content.expect("content layer should exist");
        assert!(content.start < content.end);
        assert!(content.source.contains("topic(.99)"));
    }
}
