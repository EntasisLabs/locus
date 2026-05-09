use chrono::{DateTime, Utc};

use crate::domain::models::{
    AvecState, CanonicalAst, CanonicalAstLayer, ParseDiagnostic, ParseDiagnosticSeverity,
    ParseProfile, ParseResult, ParseSpan, SttpNode,
};
use crate::parsing::lexicon::{AVEC_COMPRESSION_KEY, AVEC_MODEL_KEY, AVEC_USER_KEY};
use crate::parsing::state_machine::{ParserState, SttpLayerStateMachine};

#[derive(Debug, Clone, Copy)]
struct ContentKeySignature<'a> {
    name: &'a str,
    confidence: f32,
}

#[derive(Debug, Clone, Copy)]
enum LayerScope {
    Provenance,
    Envelope,
    Metrics,
}

impl LayerScope {
    fn name(self) -> &'static str {
        match self {
            LayerScope::Provenance => "provenance",
            LayerScope::Envelope => "envelope",
            LayerScope::Metrics => "metrics",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ObjectScope {
    ProvenancePrime,
    ProvenanceAttractorConfig,
    EnvelopeUserAvec,
    EnvelopeModelAvec,
    MetricsCompressionAvec,
}

impl ObjectScope {
    fn path(self) -> &'static str {
        match self {
            ObjectScope::ProvenancePrime => "provenance.prime",
            ObjectScope::ProvenanceAttractorConfig => "provenance.prime.attractor_config",
            ObjectScope::EnvelopeUserAvec => "envelope.user_avec",
            ObjectScope::EnvelopeModelAvec => "envelope.model_avec",
            ObjectScope::MetricsCompressionAvec => "metrics.compression_avec",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum NodeFieldKey {
    Trigger,
    ResponseFormat,
    OriginSession,
    CompressionDepth,
    ParentNode,
    Prime,
    AttractorConfig,
    ContextSummary,
    RelevantTier,
    RetrievalBudget,
    Timestamp,
    Tier,
    SessionId,
    UserAvec,
    ModelAvec,
    Stability,
    Friction,
    Logic,
    Autonomy,
    Psi,
    Rho,
    Kappa,
    CompressionAvec,
}

impl NodeFieldKey {
    fn as_str(self) -> &'static str {
        match self {
            NodeFieldKey::Trigger => "trigger",
            NodeFieldKey::ResponseFormat => "response_format",
            NodeFieldKey::OriginSession => "origin_session",
            NodeFieldKey::CompressionDepth => "compression_depth",
            NodeFieldKey::ParentNode => "parent_node",
            NodeFieldKey::Prime => "prime",
            NodeFieldKey::AttractorConfig => "attractor_config",
            NodeFieldKey::ContextSummary => "context_summary",
            NodeFieldKey::RelevantTier => "relevant_tier",
            NodeFieldKey::RetrievalBudget => "retrieval_budget",
            NodeFieldKey::Timestamp => "timestamp",
            NodeFieldKey::Tier => "tier",
            NodeFieldKey::SessionId => "session_id",
            NodeFieldKey::UserAvec => "user_avec",
            NodeFieldKey::ModelAvec => "model_avec",
            NodeFieldKey::Stability => "stability",
            NodeFieldKey::Friction => "friction",
            NodeFieldKey::Logic => "logic",
            NodeFieldKey::Autonomy => "autonomy",
            NodeFieldKey::Psi => "psi",
            NodeFieldKey::Rho => "rho",
            NodeFieldKey::Kappa => "kappa",
            NodeFieldKey::CompressionAvec => "compression_avec",
        }
    }
}

const PROVENANCE_REQUIRED_KEYS: [NodeFieldKey; 6] = [
    NodeFieldKey::Trigger,
    NodeFieldKey::ResponseFormat,
    NodeFieldKey::OriginSession,
    NodeFieldKey::CompressionDepth,
    NodeFieldKey::ParentNode,
    NodeFieldKey::Prime,
];
const PRIME_REQUIRED_KEYS: [NodeFieldKey; 4] = [
    NodeFieldKey::AttractorConfig,
    NodeFieldKey::ContextSummary,
    NodeFieldKey::RelevantTier,
    NodeFieldKey::RetrievalBudget,
];
const ATTRACTOR_REQUIRED_KEYS: [NodeFieldKey; 4] = [
    NodeFieldKey::Stability,
    NodeFieldKey::Friction,
    NodeFieldKey::Logic,
    NodeFieldKey::Autonomy,
];
const ENVELOPE_REQUIRED_KEYS: [NodeFieldKey; 5] = [
    NodeFieldKey::Timestamp,
    NodeFieldKey::Tier,
    NodeFieldKey::SessionId,
    NodeFieldKey::UserAvec,
    NodeFieldKey::ModelAvec,
];
const AVEC_REQUIRED_KEYS: [NodeFieldKey; 5] = [
    NodeFieldKey::Stability,
    NodeFieldKey::Friction,
    NodeFieldKey::Logic,
    NodeFieldKey::Autonomy,
    NodeFieldKey::Psi,
];
const METRICS_REQUIRED_KEYS: [NodeFieldKey; 4] = [
    NodeFieldKey::Rho,
    NodeFieldKey::Kappa,
    NodeFieldKey::Psi,
    NodeFieldKey::CompressionAvec,
];

trait SpecEnumValue {
    fn parse_token(input: &str) -> Option<Self>
    where
        Self: Sized;
    fn variants() -> &'static [&'static str];
}

#[derive(Debug, Clone, Copy)]
enum TriggerValue {
    Scheduled,
    Threshold,
    Resonance,
    Seed,
    Manual,
}

impl SpecEnumValue for TriggerValue {
    fn parse_token(input: &str) -> Option<Self> {
        match input.to_ascii_lowercase().as_str() {
            "scheduled" => Some(Self::Scheduled),
            "threshold" => Some(Self::Threshold),
            "resonance" => Some(Self::Resonance),
            "seed" => Some(Self::Seed),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    fn variants() -> &'static [&'static str] {
        &["scheduled", "threshold", "resonance", "seed", "manual"]
    }
}

#[derive(Debug, Clone, Copy)]
enum ResponseFormatValue {
    TemporalNode,
    NaturalLanguage,
    Hybrid,
}

impl SpecEnumValue for ResponseFormatValue {
    fn parse_token(input: &str) -> Option<Self> {
        match input.to_ascii_lowercase().as_str() {
            "temporal_node" => Some(Self::TemporalNode),
            "natural_language" => Some(Self::NaturalLanguage),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }

    fn variants() -> &'static [&'static str] {
        &["temporal_node", "natural_language", "hybrid"]
    }
}

#[derive(Debug, Clone, Copy)]
enum TierValue {
    Raw,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl SpecEnumValue for TierValue {
    fn parse_token(input: &str) -> Option<Self> {
        match input.to_ascii_lowercase().as_str() {
            "raw" => Some(Self::Raw),
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            "quarterly" => Some(Self::Quarterly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }

    fn variants() -> &'static [&'static str] {
        &["raw", "daily", "weekly", "monthly", "quarterly", "yearly"]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SttpNodeParser {
    profile: ParseProfile,
}

impl SttpNodeParser {
    pub fn new() -> Self {
        Self {
            profile: ParseProfile::Tolerant,
        }
    }

    pub fn with_profile(profile: ParseProfile) -> Self {
        Self { profile }
    }

    pub fn try_parse(&self, raw: &str, session_id: &str) -> ParseResult {
        self.try_parse_with_profile(raw, session_id, self.profile)
    }

    pub fn try_parse_strict(&self, raw: &str, session_id: &str) -> ParseResult {
        self.try_parse_with_profile(raw, session_id, ParseProfile::Strict)
    }

    pub fn try_parse_strict_typed_ir(&self, raw: &str, session_id: &str) -> ParseResult {
        self.try_parse_with_profile(raw, session_id, ParseProfile::StrictTypedIr)
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

        if requires_strict_spine(profile) && !strict_valid {
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

            if requires_strict_spine(profile) {
                return ParseResult::fail_with_metadata(
                    "strict profile violation: content schema requires field_name(.confidence): value",
                    profile,
                    diagnostics,
                    canonical_ast,
                );
            }
        }

        if requires_typed_ir_properties(profile) {
            let strict_property_diagnostics = validate_strict_required_properties(
                provenance,
                envelope,
                metrics,
                layered.provenance_span,
                layered.envelope_span,
                layered.metrics_span,
            );
            if !strict_property_diagnostics.is_empty() {
                diagnostics.extend(strict_property_diagnostics);

                return ParseResult::fail_with_metadata(
                    "strict profile violation: required typed-ir properties missing or invalid",
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
            tier: parse_tier(envelope).unwrap_or_default(),
            timestamp: parse_timestamp(envelope).unwrap_or_else(Utc::now),
            compression_depth: parse_i32_key(provenance, NodeFieldKey::CompressionDepth)
                .unwrap_or(0),
            parent_node_id: parse_parent_node(provenance),
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
            rho: parse_f32_key(metrics, NodeFieldKey::Rho).unwrap_or(0.0),
            kappa: parse_f32_key(metrics, NodeFieldKey::Kappa).unwrap_or(0.0),
            psi: parse_f32_key(metrics, NodeFieldKey::Psi).unwrap_or(0.0),
        };

        ParseResult::ok_with_metadata(node, profile, strict_valid, diagnostics, canonical_ast)
    }
}

fn requires_strict_spine(profile: ParseProfile) -> bool {
    matches!(profile, ParseProfile::Strict | ParseProfile::StrictTypedIr)
}

fn requires_typed_ir_properties(profile: ParseProfile) -> bool {
    matches!(profile, ParseProfile::StrictTypedIr)
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
    let pairs = split_top_level_pairs(object_content);
    if pairs.is_empty() {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_CONTENT_SCHEMA_EMPTY_OBJECT".to_string(),
            message: "content layer must include one or more semantic fields".to_string(),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: project_content_span(raw_node, content_layer, layer_span, object_offset, 1),
        });
        return;
    }

    for pair in pairs {
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

        let Some(signature) = parse_content_key_signature(raw_key) else {
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

        let confidence = signature.confidence;
        if !(0.0..=1.0).contains(&confidence) {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_CONTENT_SCHEMA_INVALID_CONFIDENCE".to_string(),
                message: format!(
                    "content confidence must be in [0,1]: found {confidence} for key '{}'",
                    signature.name
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

fn validate_strict_required_properties(
    provenance: &str,
    envelope: &str,
    metrics: &str,
    provenance_span: Option<crate::parsing::lexer::Span>,
    envelope_span: Option<crate::parsing::lexer::Span>,
    metrics_span: Option<crate::parsing::lexer::Span>,
) -> Vec<ParseDiagnostic> {
    let mut diagnostics = Vec::new();
    let prime_object = extract_named_object(provenance, NodeFieldKey::Prime.as_str());

    require_keys(
        provenance,
        LayerScope::Provenance,
        &PROVENANCE_REQUIRED_KEYS,
        provenance_span,
        &mut diagnostics,
    );
    require_keys(
        envelope,
        LayerScope::Envelope,
        &ENVELOPE_REQUIRED_KEYS,
        envelope_span,
        &mut diagnostics,
    );
    require_keys(
        metrics,
        LayerScope::Metrics,
        &METRICS_REQUIRED_KEYS,
        metrics_span,
        &mut diagnostics,
    );

    require_named_object_keys(
        provenance,
        NodeFieldKey::Prime,
        ObjectScope::ProvenancePrime,
        &PRIME_REQUIRED_KEYS,
        provenance_span,
        &mut diagnostics,
    );
    require_named_object_keys(
        provenance,
        NodeFieldKey::AttractorConfig,
        ObjectScope::ProvenanceAttractorConfig,
        &ATTRACTOR_REQUIRED_KEYS,
        provenance_span,
        &mut diagnostics,
    );
    require_named_object_keys(
        envelope,
        NodeFieldKey::UserAvec,
        ObjectScope::EnvelopeUserAvec,
        &AVEC_REQUIRED_KEYS,
        envelope_span,
        &mut diagnostics,
    );
    require_named_object_keys(
        envelope,
        NodeFieldKey::ModelAvec,
        ObjectScope::EnvelopeModelAvec,
        &AVEC_REQUIRED_KEYS,
        envelope_span,
        &mut diagnostics,
    );
    require_named_object_keys(
        metrics,
        NodeFieldKey::CompressionAvec,
        ObjectScope::MetricsCompressionAvec,
        &AVEC_REQUIRED_KEYS,
        metrics_span,
        &mut diagnostics,
    );

    require_typed_enum::<TriggerValue>(
        provenance,
        NodeFieldKey::Trigger,
        "provenance.trigger",
        provenance_span,
        &mut diagnostics,
    );
    require_typed_enum::<ResponseFormatValue>(
        provenance,
        NodeFieldKey::ResponseFormat,
        "provenance.response_format",
        provenance_span,
        &mut diagnostics,
    );
    require_typed_enum::<TierValue>(
        envelope,
        NodeFieldKey::Tier,
        "envelope.tier",
        envelope_span,
        &mut diagnostics,
    );
    require_typed_enum_in_optional_object::<TierValue>(
        prime_object,
        NodeFieldKey::RelevantTier,
        "provenance.prime.relevant_tier",
        provenance_span,
        &mut diagnostics,
    );

    require_numeric(
        provenance,
        NodeFieldKey::CompressionDepth,
        "provenance.compression_depth",
        NumericKind::Integer,
        provenance_span,
        &mut diagnostics,
    );
    require_numeric_in_optional_object(
        prime_object,
        NodeFieldKey::RetrievalBudget,
        "provenance.prime.retrieval_budget",
        NumericKind::Integer,
        provenance_span,
        &mut diagnostics,
    );
    require_numeric(
        metrics,
        NodeFieldKey::Rho,
        "metrics.rho",
        NumericKind::Float,
        metrics_span,
        &mut diagnostics,
    );
    require_numeric(
        metrics,
        NodeFieldKey::Kappa,
        "metrics.kappa",
        NumericKind::Float,
        metrics_span,
        &mut diagnostics,
    );
    require_numeric(
        metrics,
        NodeFieldKey::Psi,
        "metrics.psi",
        NumericKind::Float,
        metrics_span,
        &mut diagnostics,
    );

    diagnostics
}

fn require_typed_enum_in_optional_object<E: SpecEnumValue>(
    source: Option<&str>,
    key: NodeFieldKey,
    path: &str,
    span: Option<crate::parsing::lexer::Span>,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    let Some(source) = source else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_OBJECT".to_string(),
            message: "missing required object 'provenance.prime'".to_string(),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    let Some(value) = parse_scalar_token_in_object(source, key) else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_KEY".to_string(),
            message: format!("missing required enum key '{path}'"),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    if E::parse_token(&value).is_none() {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_INVALID_ENUM".to_string(),
            message: format!(
                "invalid enum value for {path}: '{value}' (expected one of: {})",
                E::variants().join("|")
            ),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
    }
}

fn require_keys(
    source: &str,
    layer: LayerScope,
    keys: &[NodeFieldKey],
    span: Option<crate::parsing::lexer::Span>,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    for key in keys {
        if !contains_key_in_layer(source, *key) {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_STRICT_MISSING_REQUIRED_KEY".to_string(),
                message: format!(
                    "missing required key '{}' in {} layer",
                    key.as_str(),
                    layer.name()
                ),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: span.map(to_parse_span),
            });
        }
    }
}

fn require_named_object_keys(
    source: &str,
    object_key: NodeFieldKey,
    path: ObjectScope,
    keys: &[NodeFieldKey],
    span: Option<crate::parsing::lexer::Span>,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    let Some(object) = extract_named_object(source, object_key.as_str()) else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_OBJECT".to_string(),
            message: format!("missing required object '{}'", path.path()),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    for key in keys {
        if !contains_key_in_object(object, *key) {
            diagnostics.push(ParseDiagnostic {
                code: "STTP_STRICT_MISSING_REQUIRED_KEY".to_string(),
                message: format!("missing required key '{}' in {}", key.as_str(), path.path()),
                severity: ParseDiagnosticSeverity::Error,
                strict_impact: true,
                span: span.map(to_parse_span),
            });
        }
    }
}

fn require_typed_enum<E: SpecEnumValue>(
    source: &str,
    key: NodeFieldKey,
    path: &str,
    span: Option<crate::parsing::lexer::Span>,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    let Some(value) = parse_scalar_token_in_layer(source, key) else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_KEY".to_string(),
            message: format!("missing required enum key '{path}'"),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    if E::parse_token(&value).is_none() {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_INVALID_ENUM".to_string(),
            message: format!(
                "invalid enum value for {path}: '{value}' (expected one of: {})",
                E::variants().join("|")
            ),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
    }
}

#[derive(Debug, Clone, Copy)]
enum NumericKind {
    Integer,
    Float,
}

fn require_numeric(
    source: &str,
    key: NodeFieldKey,
    path: &str,
    kind: NumericKind,
    span: Option<crate::parsing::lexer::Span>,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    let Some(value) = parse_scalar_token_in_layer(source, key) else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_KEY".to_string(),
            message: format!("missing required numeric key '{path}'"),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    let numeric_ok = if matches!(kind, NumericKind::Integer) {
        value.parse::<i64>().is_ok()
    } else {
        value.parse::<f64>().is_ok()
    };

    if !numeric_ok {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_INVALID_NUMERIC".to_string(),
            message: format!("invalid numeric value for {path}: '{value}'"),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
    }
}

fn require_numeric_in_optional_object(
    source: Option<&str>,
    key: NodeFieldKey,
    path: &str,
    kind: NumericKind,
    span: Option<crate::parsing::lexer::Span>,
    diagnostics: &mut Vec<ParseDiagnostic>,
) {
    let Some(source) = source else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_OBJECT".to_string(),
            message: "missing required object 'provenance.prime'".to_string(),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    let Some(value) = parse_scalar_token_in_object(source, key) else {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_MISSING_REQUIRED_KEY".to_string(),
            message: format!("missing required numeric key '{path}'"),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
        return;
    };

    let numeric_ok = if matches!(kind, NumericKind::Integer) {
        value.parse::<i64>().is_ok()
    } else {
        value.parse::<f64>().is_ok()
    };

    if !numeric_ok {
        diagnostics.push(ParseDiagnostic {
            code: "STTP_STRICT_INVALID_NUMERIC".to_string(),
            message: format!("invalid numeric value for {path}: '{value}'"),
            severity: ParseDiagnosticSeverity::Error,
            strict_impact: true,
            span: span.map(to_parse_span),
        });
    }
}

fn contains_key_in_layer(source: &str, key: NodeFieldKey) -> bool {
    parse_scalar_token_in_layer(source, key).is_some()
}

fn contains_key_in_object(source: &str, key: NodeFieldKey) -> bool {
    parse_scalar_token_in_object(source, key).is_some()
}

fn parse_scalar_token_in_layer(source: &str, key: NodeFieldKey) -> Option<String> {
    let object = extract_first_object(source)?;
    parse_scalar_token_in_object(object, key)
}

fn parse_scalar_token_in_object(source: &str, key: NodeFieldKey) -> Option<String> {
    parse_key_value_in_object(source, key.as_str()).map(normalize_scalar_value)
}

fn parse_key_value_in_object<'a>(object: &'a str, key: &str) -> Option<&'a str> {
    for pair in split_top_level_pairs(object) {
        let Some(colon_idx) = find_top_level_colon(pair.text) else {
            continue;
        };
        let raw_key = pair.text[..colon_idx].trim();
        let normalized = normalize_key(raw_key);
        if normalized.eq_ignore_ascii_case(key) {
            return Some(pair.text[colon_idx + 1..].trim());
        }
    }

    None
}

fn normalize_key(raw_key: &str) -> &str {
    raw_key
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(raw_key)
        .trim()
}

fn normalize_scalar_value(raw_value: &str) -> String {
    let trimmed = raw_value.trim();
    if let Some(unquoted) = trimmed.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return unquoted.trim().to_string();
    }

    trimmed.to_string()
}

fn parse_content_key_signature(raw_key: &str) -> Option<ContentKeySignature<'_>> {
    let open = raw_key.find('(')?;
    let close = raw_key.rfind(')')?;
    if close <= open + 1 {
        return None;
    }

    let name = raw_key[..open].trim();
    if name.is_empty() || !is_valid_identifier(name) {
        return None;
    }

    let confidence_text = raw_key[open + 1..close].trim();
    let confidence = confidence_text.parse::<f32>().ok()?;

    Some(ContentKeySignature { name, confidence })
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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
    let stability = parse_f32_key_in_object(object, NodeFieldKey::Stability);
    let friction = parse_f32_key_in_object(object, NodeFieldKey::Friction);
    let logic = parse_f32_key_in_object(object, NodeFieldKey::Logic);
    let autonomy = parse_f32_key_in_object(object, NodeFieldKey::Autonomy);

    Some(AvecState {
        stability: stability?,
        friction: friction?,
        logic: logic?,
        autonomy: autonomy?,
    })
}

fn parse_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    let maybe_ts = parse_scalar_token_in_layer(raw, NodeFieldKey::Timestamp);

    if let Some(ts) = maybe_ts {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(&ts) {
            return Some(parsed.with_timezone(&Utc));
        }
    }

    None
}

fn parse_tier(raw: &str) -> Option<String> {
    parse_scalar_token_in_layer(raw, NodeFieldKey::Tier)
}

fn parse_parent_node(raw: &str) -> Option<String> {
    let value = parse_scalar_token_in_layer(raw, NodeFieldKey::ParentNode)?;
    if value.eq_ignore_ascii_case("null") {
        return None;
    }

    if let Some(reference) = value.strip_prefix("ref:") {
        return Some(reference.trim().to_string());
    }

    Some(value)
}

fn parse_context_summary(raw: &str) -> Option<String> {
    let prime = extract_named_object(raw, NodeFieldKey::Prime.as_str())?;
    let value = parse_scalar_token_in_object(prime, NodeFieldKey::ContextSummary)?;

    if value.is_empty() { None } else { Some(value) }
}

fn parse_i32_key(source: &str, key: NodeFieldKey) -> Option<i32> {
    parse_scalar_token_in_layer(source, key).and_then(|v| v.parse::<i32>().ok())
}

fn parse_f32_key(source: &str, key: NodeFieldKey) -> Option<f32> {
    parse_scalar_token_in_layer(source, key).and_then(|v| v.parse::<f32>().ok())
}

fn parse_f32_key_in_object(source: &str, key: NodeFieldKey) -> Option<f32> {
    parse_scalar_token_in_object(source, key).and_then(|v| v.parse::<f32>().ok())
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

    #[test]
    fn strict_profile_should_fail_when_required_property_is_missing() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, context_summary: "missing origin_session", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "strict-test", user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
◈⟨ { note(.99): "ok" } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
"#;

        let parsed = parser.try_parse_strict_typed_ir(raw, "strict-required");
        assert!(!parsed.success);
        assert_eq!(parsed.profile, ParseProfile::StrictTypedIr);
        assert!(parsed.diagnostics.iter().any(|d| {
            d.code == "STTP_STRICT_MISSING_REQUIRED_KEY" && d.message.contains("origin_session")
        }));
    }

    #[test]
    fn strict_profile_should_fail_on_invalid_enum_value() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "strict-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, context_summary: "bad tier", relevant_tier: badtier, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "strict-test", user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
◈⟨ { note(.99): "ok" } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
"#;

        let parsed = parser.try_parse_strict_typed_ir(raw, "strict-enum");
        assert!(!parsed.success);
        assert_eq!(parsed.profile, ParseProfile::StrictTypedIr);
        assert!(parsed.diagnostics.iter().any(|d| {
            d.code == "STTP_STRICT_INVALID_ENUM"
                || (d.code == "STTP_STRICT_MISSING_REQUIRED_KEY"
                    && d.message.contains("relevant_tier"))
        }));
    }

    #[test]
    fn strict_profile_should_fail_when_content_object_is_empty() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "strict-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, context_summary: "empty content", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "strict-test", user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
◈⟨ { } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
"#;

        let parsed = parser.try_parse_strict_typed_ir(raw, "strict-empty-content");
        assert!(!parsed.success);
        assert_eq!(parsed.profile, ParseProfile::StrictTypedIr);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d.code == "STTP_CONTENT_SCHEMA_EMPTY_OBJECT")
        );
    }

    #[test]
    fn strict_profile_without_typed_ir_should_not_require_origin_session() {
        let parser = SttpNodeParser::new();
        let raw = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7 }, context_summary: "legacy strict", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "strict-test", user_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 }, model_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
◈⟨ { note(.99): "ok" } ⟩
⍉⟨ { rho: 0.1, kappa: 0.2, psi: 2.6, compression_avec: { stability: 0.8, friction: 0.2, logic: 0.9, autonomy: 0.7, psi: 2.6 } } ⟩
"#;

        let parsed = parser.try_parse_strict(raw, "strict-legacy");
        assert!(parsed.success);
        assert_eq!(parsed.profile, ParseProfile::Strict);
    }
}
