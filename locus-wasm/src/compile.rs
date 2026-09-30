//! Turn a plain note into a canonical STTP record for the in-browser demo.
//!
//! Compression picks a short summary and tags. The note itself stays in the
//! content layer so lexical recall can match the words the person typed.

use locus_core_rs::domain::models::AvecState;
use locus_core_rs::parsing::{SttpContentSlice, SttpDocumentBuilder, SttpDocumentMetadata};
use locus_sdk::application::manual_compression::{
    DefaultManualCompressionLexiconProvider, ManualCompressionService,
};
use locus_sdk::domain::compression::ManualCompressionRequest;
use serde::Serialize;
use serde_json::json;

const MAX_NOTE_CHARS: usize = 4_000;
const MAX_SUMMARY_CHARS: usize = 180;
#[cfg_attr(not(test), allow(dead_code))]
pub const DEMO_SESSION_ID: &str = "homepage-demo";

/// Same scores the site uses in its examples. Psi is their sum.
const DEMO_AVEC: AvecState = AvecState {
    stability: 0.85,
    friction: 0.25,
    logic: 0.80,
    autonomy: 0.70,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledNote {
    pub canonical: String,
    pub session_id: String,
    pub context_summary: String,
    pub anchor_topic: String,
    pub anchor_terms: Vec<String>,
    pub key_points: Vec<String>,
}

pub fn compile_note(text: &str, session_id: &str) -> Result<CompiledNote, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Write a note before saving it.".to_string());
    }
    if text.chars().count() > MAX_NOTE_CHARS {
        return Err(format!(
            "Keep the note under {MAX_NOTE_CHARS} characters for this demo."
        ));
    }

    let session_id = session_id.trim();
    if session_id.is_empty() {
        return Err("A session id is required.".to_string());
    }

    let compression =
        ManualCompressionService::with_lexicon_provider(DefaultManualCompressionLexiconProvider);
    let compressed = compression.execute(&ManualCompressionRequest {
        text: text.to_string(),
        max_anchors: 5,
        max_points: 4,
        ..ManualCompressionRequest::default()
    });

    let context_summary = summary_text(&compressed.anchor_topic, text);
    let tags = semantic_tags(&compressed.anchor_terms);
    let key_points = compressed.key_points;

    let mut metadata = SttpDocumentMetadata::new(session_id)
        .with_context_summary(context_summary.clone())
        .with_avec(DEMO_AVEC, DEMO_AVEC);
    if !tags.is_empty() {
        metadata = metadata.with_semantic_tags(tags.clone());
    }

    let mut slice = SttpContentSlice::new()
        .field("note", 0.99, json!(text))
        .map_err(|err| err.to_string())?;
    if !key_points.is_empty() {
        let points = key_points.join("; ");
        slice = slice
            .field("points", 0.90, json!(points))
            .map_err(|err| err.to_string())?;
    }

    let document = SttpDocumentBuilder::new(metadata)
        .merge(slice)
        .map_err(|err| err.to_string())?
        .build()
        .map_err(|err| err.to_string())?;

    Ok(CompiledNote {
        canonical: document.render_canonical(),
        session_id: session_id.to_string(),
        context_summary,
        anchor_topic: compressed.anchor_topic,
        anchor_terms: tags,
        key_points,
    })
}

fn summary_text(anchor_topic: &str, text: &str) -> String {
    let source = if anchor_topic.trim().is_empty() {
        text
    } else {
        anchor_topic
    };
    let mut summary = String::new();
    for (index, ch) in source.trim().chars().enumerate() {
        if index >= MAX_SUMMARY_CHARS {
            break;
        }
        summary.push(ch);
    }
    if summary.is_empty() {
        "note".to_string()
    } else {
        summary
    }
}

fn semantic_tags(terms: &[locus_sdk::domain::compression::AnchorTerm]) -> Vec<String> {
    let mut tags = Vec::new();
    for term in terms {
        let tag = sanitize_tag(&term.term);
        if tag.len() < 2 || tags.iter().any(|existing: &String| existing == &tag) {
            continue;
        }
        tags.push(tag);
        if tags.len() == 5 {
            break;
        }
    }
    tags
}

fn sanitize_tag(term: &str) -> String {
    let mut tag = String::new();
    let mut pending_hyphen = false;
    for ch in term.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_hyphen && !tag.is_empty() {
                tag.push('-');
            }
            pending_hyphen = false;
            tag.push(ch.to_ascii_lowercase());
        } else if !tag.is_empty() {
            pending_hyphen = true;
        }
    }
    tag
}

#[cfg(test)]
mod tests {
    use super::*;
    use locus_core_rs::application::validation::TreeSitterValidator;
    use locus_core_rs::domain::contracts::NodeValidator;
    use locus_core_rs::parsing::SttpNodeParser;

    #[test]
    fn compile_note_emits_a_valid_record() {
        let compiled = compile_note(
            "We decided the parser should accept both strict and tolerant STTP.",
            DEMO_SESSION_ID,
        )
        .expect("compile");

        assert!(compiled.canonical.contains("note(.99)"));
        assert!(compiled.canonical.contains("strict"));
        assert_eq!(compiled.session_id, DEMO_SESSION_ID);
        assert!(!compiled.context_summary.is_empty());

        let validation = TreeSitterValidator::new().validate(&compiled.canonical);
        assert!(validation.is_valid, "{:?}", validation.error);

        let parsed = SttpNodeParser::new().try_parse(&compiled.canonical, DEMO_SESSION_ID);
        assert!(parsed.success, "{:?}", parsed.error);
        assert!(parsed.node.is_some());
    }

    #[test]
    fn compile_note_rejects_blank_text() {
        let error = compile_note("   ", DEMO_SESSION_ID).unwrap_err();
        assert!(error.contains("note"));
    }
}
