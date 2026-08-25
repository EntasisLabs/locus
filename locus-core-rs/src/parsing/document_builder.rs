//! Fluent STTP document construction with shallow content-layer merge.
//!
//! Merge is intentionally limited: slices may only contribute **top-level content
//! keys**. Provenance and envelope come from metadata; metrics are finalized at
//! [`SttpDocumentBuilder::build`]. Nested objects inside a top-level field are
//! opaque and are never deep-merged.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::domain::models::{AvecState, SemanticLink};

/// Seed data for provenance + envelope layers.
#[derive(Debug, Clone)]
pub struct SttpDocumentMetadata {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub tier: String,
    pub trigger: String,
    pub response_format: String,
    pub compression_depth: i32,
    pub parent_node: Option<String>,
    pub context_summary: String,
    pub retrieval_budget: i32,
    pub attractor_config: AvecState,
    pub user_avec: AvecState,
    pub model_avec: AvecState,
    pub semantic_tags: Option<Vec<String>>,
    pub semantic_links: Option<Vec<SemanticLink>>,
    pub schema_version: Option<String>,
    /// Optional metrics overrides; unset fields are derived at build time.
    pub rho: Option<f32>,
    pub kappa: Option<f32>,
    pub compression_avec: Option<AvecState>,
}

impl SttpDocumentMetadata {
    pub fn new(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let attractor = AvecState::analytical();
        Self {
            session_id,
            timestamp: Utc::now(),
            tier: "raw".to_string(),
            trigger: "manual".to_string(),
            response_format: "temporal_node".to_string(),
            compression_depth: 1,
            parent_node: None,
            context_summary: String::new(),
            retrieval_budget: 5,
            attractor_config: attractor,
            user_avec: attractor,
            model_avec: attractor,
            semantic_tags: None,
            semantic_links: None,
            schema_version: Some("sttp-1.2".to_string()),
            rho: None,
            kappa: None,
            compression_avec: None,
        }
    }

    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_tier(mut self, tier: impl Into<String>) -> Self {
        self.tier = tier.into();
        self
    }

    pub fn with_context_summary(mut self, summary: impl Into<String>) -> Self {
        self.context_summary = summary.into();
        self
    }

    pub fn with_avec(mut self, user: AvecState, model: AvecState) -> Self {
        self.user_avec = user;
        self.model_avec = model;
        self.attractor_config = user;
        self
    }

    pub fn with_semantic_tags(mut self, tags: Vec<String>) -> Self {
        self.semantic_tags = Some(tags);
        self
    }

    pub fn with_semantic_links(mut self, links: Vec<SemanticLink>) -> Self {
        self.semantic_links = Some(links);
        self
    }
}

/// A content-layer contribution limited to top-level keys.
///
/// Keys must use the canonical confidence signature: `field_name(.confidence)`.
#[derive(Debug, Clone, Default)]
pub struct SttpContentSlice {
    fields: Map<String, Value>,
}

impl SttpContentSlice {
    pub fn new() -> Self {
        Self {
            fields: Map::new(),
        }
    }

    /// Insert a top-level content field with an explicit confidence annotation.
    pub fn field(
        mut self,
        name: impl Into<String>,
        confidence: f32,
        value: Value,
    ) -> Result<Self, SttpDocumentBuildError> {
        let name = name.into();
        validate_identifier(&name)?;
        validate_confidence(confidence)?;
        let key = format_content_key(&name, confidence);
        if content_field_name_occupied(&self.fields, &name) {
            return Err(SttpDocumentBuildError::DuplicateContentField(name));
        }
        self.fields.insert(key, value);
        Ok(self)
    }

    /// Build a slice from a map whose keys already include confidence signatures.
    pub fn from_confidence_map(map: Map<String, Value>) -> Result<Self, SttpDocumentBuildError> {
        let mut slice = Self::new();
        for (key, value) in map {
            let name = parse_content_field_name(&key).ok_or_else(|| {
                SttpDocumentBuildError::InvalidContentKey(key.clone())
            })?;
            if content_field_name_occupied(&slice.fields, name) {
                return Err(SttpDocumentBuildError::DuplicateContentField(name.to_string()));
            }
            slice.fields.insert(key, value);
        }
        Ok(slice)
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }
}

/// Fluent builder: metadata → shallow content merges → built document.
#[derive(Debug, Clone)]
pub struct SttpDocumentBuilder {
    metadata: SttpDocumentMetadata,
    /// Top-level content fields keyed by full `name(.confidence)` string.
    content: Map<String, Value>,
    /// Field names (without confidence) already claimed, for collision checks.
    claimed_names: BTreeMap<String, String>,
}

impl SttpDocumentBuilder {
    pub fn new(metadata: SttpDocumentMetadata) -> Self {
        Self {
            metadata,
            content: Map::new(),
            claimed_names: BTreeMap::new(),
        }
    }

    /// Shallow-merge a content slice at the top-level key layer only.
    ///
    /// Duplicate field **names** (ignoring confidence) are rejected.
    pub fn merge(mut self, slice: SttpContentSlice) -> Result<Self, SttpDocumentBuildError> {
        for (key, value) in slice.fields {
            let name = parse_content_field_name(&key)
                .ok_or_else(|| SttpDocumentBuildError::InvalidContentKey(key.clone()))?
                .to_string();
            if let Some(existing_key) = self.claimed_names.get(&name) {
                return Err(SttpDocumentBuildError::DuplicateContentField(format!(
                    "{name} (already present as {existing_key})"
                )));
            }
            self.claimed_names.insert(name, key.clone());
            self.content.insert(key, value);
        }
        Ok(self)
    }

    /// Finalize metrics and produce a document ready for canonical render.
    pub fn build(self) -> Result<SttpDocument, SttpDocumentBuildError> {
        validate_metadata(&self.metadata)?;
        if self.content.is_empty() {
            return Err(SttpDocumentBuildError::EmptyContent);
        }

        let compression_avec = self
            .metadata
            .compression_avec
            .unwrap_or(self.metadata.user_avec);
        let psi = compression_avec.psi();
        let rho = self.metadata.rho.unwrap_or(0.95);
        let kappa = self.metadata.kappa.unwrap_or(0.94);
        validate_confidence(rho)?;
        validate_confidence(kappa)?;

        Ok(SttpDocument {
            metadata: self.metadata,
            content: self.content,
            rho,
            kappa,
            psi,
            compression_avec,
        })
    }
}

/// Built STTP document: structured layers ready for canonical rendering.
#[derive(Debug, Clone)]
pub struct SttpDocument {
    metadata: SttpDocumentMetadata,
    content: Map<String, Value>,
    rho: f32,
    kappa: f32,
    psi: f32,
    compression_avec: AvecState,
}

impl SttpDocument {
    pub fn content(&self) -> &Map<String, Value> {
        &self.content
    }

    pub fn metadata(&self) -> &SttpDocumentMetadata {
        &self.metadata
    }

    /// Emit strict-profile canonical STTP wire text (four-layer spine).
    pub fn render_canonical(&self) -> String {
        let provenance = render_provenance(&self.metadata);
        let envelope = render_envelope(&self.metadata);
        let content = render_sttp_object(&self.content);
        let metrics = render_metrics(self.rho, self.kappa, self.psi, self.compression_avec);

        format!(
            "⊕⟨ {provenance} ⟩\n⦿⟨ {envelope} ⟩\n◈⟨ {content} ⟩\n⍉⟨ {metrics} ⟩"
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SttpDocumentBuildError {
    EmptyContent,
    DuplicateContentField(String),
    InvalidContentKey(String),
    InvalidIdentifier(String),
    InvalidConfidence(String),
    InvalidMetadata(String),
}

impl fmt::Display for SttpDocumentBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyContent => write!(f, "content layer must contain at least one field"),
            Self::DuplicateContentField(name) => {
                write!(f, "duplicate top-level content field: {name}")
            }
            Self::InvalidContentKey(key) => {
                write!(
                    f,
                    "content key must match field_name(.confidence): found '{key}'"
                )
            }
            Self::InvalidIdentifier(name) => {
                write!(f, "invalid content field identifier: '{name}'")
            }
            Self::InvalidConfidence(detail) => write!(f, "invalid confidence: {detail}"),
            Self::InvalidMetadata(detail) => write!(f, "invalid document metadata: {detail}"),
        }
    }
}

impl std::error::Error for SttpDocumentBuildError {}

fn validate_metadata(meta: &SttpDocumentMetadata) -> Result<(), SttpDocumentBuildError> {
    if meta.session_id.trim().is_empty() {
        return Err(SttpDocumentBuildError::InvalidMetadata(
            "session_id must be non-empty".to_string(),
        ));
    }
    if meta.context_summary.is_empty() {
        return Err(SttpDocumentBuildError::InvalidMetadata(
            "context_summary must be non-empty".to_string(),
        ));
    }
    validate_enum(
        "trigger",
        &meta.trigger,
        &["scheduled", "threshold", "resonance", "seed", "manual"],
    )?;
    validate_enum(
        "response_format",
        &meta.response_format,
        &["temporal_node", "natural_language", "hybrid"],
    )?;
    validate_enum(
        "tier",
        &meta.tier,
        &["raw", "daily", "weekly", "monthly", "quarterly", "yearly"],
    )?;
    validate_enum(
        "relevant_tier",
        &meta.tier,
        &["raw", "daily", "weekly", "monthly", "quarterly", "yearly"],
    )?;
    if let Some(tags) = &meta.semantic_tags {
        if tags.is_empty() {
            return Err(SttpDocumentBuildError::InvalidMetadata(
                "semantic_tags when present must be non-empty".to_string(),
            ));
        }
    }
    if let Some(links) = &meta.semantic_links {
        if links.is_empty() {
            return Err(SttpDocumentBuildError::InvalidMetadata(
                "semantic_links when present must be non-empty".to_string(),
            ));
        }
        for link in links {
            if link.rel.trim().is_empty() || link.target.trim().is_empty() {
                return Err(SttpDocumentBuildError::InvalidMetadata(
                    "semantic_links entries require non-empty rel and target".to_string(),
                ));
            }
            if let Some(confidence) = link.confidence {
                validate_confidence(confidence)?;
            }
        }
    }
    Ok(())
}

fn validate_enum(
    field: &str,
    value: &str,
    allowed: &[&str],
) -> Result<(), SttpDocumentBuildError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(SttpDocumentBuildError::InvalidMetadata(format!(
            "{field} must be one of {:?}, found '{value}'",
            allowed
        )))
    }
}

fn validate_identifier(name: &str) -> Result<(), SttpDocumentBuildError> {
    let valid = !name.is_empty()
        && name
            .chars()
            .enumerate()
            .all(|(idx, ch)| match idx {
                0 => ch.is_ascii_alphabetic() || ch == '_',
                _ => ch.is_ascii_alphanumeric() || ch == '_',
            });
    if valid {
        Ok(())
    } else {
        Err(SttpDocumentBuildError::InvalidIdentifier(name.to_string()))
    }
}

fn validate_confidence(value: f32) -> Result<(), SttpDocumentBuildError> {
    if (0.0..=1.0).contains(&value) && value.is_finite() {
        Ok(())
    } else {
        Err(SttpDocumentBuildError::InvalidConfidence(format!(
            "expected [0.0, 1.0], found {value}"
        )))
    }
}

fn format_content_key(name: &str, confidence: f32) -> String {
    // Canonical form matches language fixtures: field(.98)
    let text = format!("{confidence:.2}");
    let body = text
        .strip_prefix("0.")
        .map(|rest| format!(".{rest}"))
        .unwrap_or(text);
    format!("{name}({body})")
}

fn parse_content_field_name(raw_key: &str) -> Option<&str> {
    let open = raw_key.find('(')?;
    let close = raw_key.rfind(')')?;
    if close <= open + 1 || close != raw_key.len() - 1 {
        return None;
    }
    let name = raw_key[..open].trim();
    if name.is_empty() {
        return None;
    }
    let confidence_text = raw_key[open + 1..close].trim();
    let confidence = confidence_text.parse::<f32>().ok()?;
    if !(0.0..=1.0).contains(&confidence) {
        return None;
    }
    Some(name)
}

fn content_field_name_occupied(fields: &Map<String, Value>, name: &str) -> bool {
    fields.keys().any(|key| parse_content_field_name(key) == Some(name))
}

fn render_provenance(meta: &SttpDocumentMetadata) -> String {
    let parent = match &meta.parent_node {
        Some(id) => format!("\"{}\"", escape_string(id)),
        None => "null".to_string(),
    };
    let attractor = render_avec_body(meta.attractor_config, false);
    let mut prime = format!(
        "{{ attractor_config: {{ {attractor} }}, context_summary: \"{}\", relevant_tier: {}, retrieval_budget: {}",
        escape_string(&meta.context_summary),
        meta.tier,
        meta.retrieval_budget
    );
    if let Some(tags) = &meta.semantic_tags {
        let rendered = canonicalize_tags(tags)
            .into_iter()
            .map(|tag| format!("\"{}\"", escape_string(&tag)))
            .collect::<Vec<_>>()
            .join(", ");
        prime.push_str(&format!(", semantic_tags: [{rendered}]"));
    }
    prime.push_str(" }");

    let mut body = format!(
        "{{ trigger: {}, response_format: {}, origin_session: \"{}\", compression_depth: {}, parent_node: {}, prime: {prime}",
        meta.trigger,
        meta.response_format,
        escape_string(&meta.session_id),
        meta.compression_depth,
        parent
    );
    if let Some(links) = &meta.semantic_links {
        let rendered = links
            .iter()
            .map(render_semantic_link)
            .collect::<Vec<_>>()
            .join(", ");
        body.push_str(&format!(", semantic_links: [{rendered}]"));
    }
    body.push_str(" }");
    body
}

fn render_envelope(meta: &SttpDocumentMetadata) -> String {
    let user = render_avec_body(meta.user_avec, true);
    let model = render_avec_body(meta.model_avec, true);
    let mut body = format!(
        "{{ timestamp: \"{}\", tier: {}, session_id: \"{}\"",
        meta.timestamp.to_rfc3339(),
        meta.tier,
        escape_string(&meta.session_id)
    );
    if let Some(version) = &meta.schema_version {
        body.push_str(&format!(
            ", schema_version: \"{}\"",
            escape_string(version)
        ));
    }
    body.push_str(&format!(
        ", user_avec: {{ {user} }}, model_avec: {{ {model} }} }}"
    ));
    body
}

fn render_metrics(rho: f32, kappa: f32, psi: f32, compression_avec: AvecState) -> String {
    let avec = render_avec_body(compression_avec, true);
    format!(
        "{{ rho: {}, kappa: {}, psi: {}, compression_avec: {{ {avec} }} }}",
        format_float(rho),
        format_float(kappa),
        format_float(psi)
    )
}

fn render_avec_body(avec: AvecState, include_psi: bool) -> String {
    if include_psi {
        format!(
            "stability: {}, friction: {}, logic: {}, autonomy: {}, psi: {}",
            format_float(avec.stability),
            format_float(avec.friction),
            format_float(avec.logic),
            format_float(avec.autonomy),
            format_float(avec.psi())
        )
    } else {
        format!(
            "stability: {}, friction: {}, logic: {}, autonomy: {}",
            format_float(avec.stability),
            format_float(avec.friction),
            format_float(avec.logic),
            format_float(avec.autonomy)
        )
    }
}

fn render_semantic_link(link: &SemanticLink) -> String {
    let mut body = format!(
        "{{ rel: \"{}\", target: \"{}\"",
        escape_string(&link.rel),
        escape_string(&link.target)
    );
    if let Some(confidence) = link.confidence {
        body.push_str(&format!(", confidence: {}", format_float(confidence)));
    }
    body.push_str(" }");
    body
}

fn render_sttp_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => format!("\"{}\"", escape_string(v)),
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

fn canonicalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = tags
        .iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn escape_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_float(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.trunc()).abs() < f32::EPSILON {
        format!("{:.1}", rounded)
    } else {
        let text = format!("{rounded}");
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::validation::TreeSitterValidator;
    use crate::domain::contracts::NodeValidator;
    use crate::parsing::SttpNodeParser;
    use chrono::TimeZone;
    use serde_json::json;

    fn sample_metadata() -> SttpDocumentMetadata {
        SttpDocumentMetadata::new("builder-session")
            .with_timestamp(
                Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0)
                    .single()
                    .expect("valid timestamp"),
            )
            .with_context_summary("document builder smoke")
            .with_avec(AvecState::analytical(), AvecState::analytical())
    }

    #[test]
    fn merge_rejects_duplicate_top_level_field_names() {
        let core = SttpContentSlice::new()
            .field("core", 0.98, json!({"note(.99)": "a"}))
            .expect("core slice");
        let conflict = SttpContentSlice::new()
            .field("core", 0.50, json!({"note(.99)": "b"}))
            .expect("conflict slice");

        let result = SttpDocumentBuilder::new(sample_metadata())
            .merge(core)
            .expect("first merge")
            .merge(conflict);

        assert!(matches!(
            result,
            Err(SttpDocumentBuildError::DuplicateContentField(_))
        ));
    }

    #[test]
    fn build_requires_non_empty_content() {
        let result = SttpDocumentBuilder::new(sample_metadata()).build();
        assert_eq!(result.unwrap_err(), SttpDocumentBuildError::EmptyContent);
    }

    #[test]
    fn fluent_merge_build_render_round_trips_strict_typed_ir() {
        let metadata = sample_metadata()
            .with_semantic_tags(vec!["Core".to_string(), "parser".to_string()])
            .with_semantic_links(vec![SemanticLink {
                rel: "related_to".to_string(),
                target: "concept:document-builder".to_string(),
                confidence: Some(0.88),
            }]);

        let core = SttpContentSlice::new()
            .field(
                "core",
                0.98,
                json!({
                    "focus(.99)": "grammar",
                    "decision(.96)": { "parser_mode(.95)": "strict_and_tolerant" }
                }),
            )
            .expect("core");
        let mode = SttpContentSlice::new()
            .field("mode", 0.97, json!({ "profile(.99)": "strict" }))
            .expect("mode");
        let turn = SttpContentSlice::new()
            .field("turn", 0.96, json!({ "utterance(.90)": "lock merge at top level" }))
            .expect("turn");

        let rendered = SttpDocumentBuilder::new(metadata)
            .merge(core)
            .expect("merge core")
            .merge(mode)
            .expect("merge mode")
            .merge(turn)
            .expect("merge turn")
            .build()
            .expect("build")
            .render_canonical();

        let validator = TreeSitterValidator::new();
        let validation = validator.validate(&rendered);
        assert!(validation.is_valid, "{:?}", validation.error);

        let parsed = SttpNodeParser::new().try_parse_strict_typed_ir(&rendered, "builder-session");
        assert!(parsed.success, "{:?}", parsed.error);
        assert!(parsed.strict_valid);

        let node = parsed.node.expect("node");
        assert_eq!(
            node.semantic_tags,
            Some(vec!["core".to_string(), "parser".to_string()])
        );
        assert!(rendered.contains("core(.98):"));
        assert!(rendered.contains("mode(.97):"));
        assert!(rendered.contains("turn(.96):"));
    }
}
