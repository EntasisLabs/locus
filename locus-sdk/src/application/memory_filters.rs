use std::collections::HashSet;

use locus_core_rs::domain::models::SttpNode;

use crate::domain::memory::{MemoryFilter, MemoryScope};

pub fn build_session_filter(scope: &MemoryScope) -> Option<HashSet<String>> {
    scope
        .session_ids
        .as_ref()
        .map(|sessions| sessions.iter().map(|s| s.to_ascii_lowercase()).collect())
}

pub fn node_matches_common_filters(
    node: &SttpNode,
    scope: &MemoryScope,
    filter: &MemoryFilter,
    session_filter: Option<&HashSet<String>>,
) -> bool {
    let _ = scope;

    if let Some(sessions) = session_filter
        && !sessions.contains(&node.session_id.to_ascii_lowercase())
    {
        return false;
    }

    if let Some(expected) = filter.has_embedding {
        let has_embedding = node.embedding.as_ref().is_some_and(|values| !values.is_empty());
        if has_embedding != expected {
            return false;
        }
    }

    if let Some(expected_model) = filter.embedding_model.as_deref() {
        let expected = expected_model.trim().to_ascii_lowercase();
        let actual = node
            .embedding_model
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if expected != actual {
            return false;
        }
    }

    if let Some(range) = &filter.psi
        && !range.contains(node.psi)
    {
        return false;
    }

    if let Some(range) = &filter.rho
        && !range.contains(node.rho)
    {
        return false;
    }

    if let Some(range) = &filter.kappa
        && !range.contains(node.kappa)
    {
        return false;
    }

    if let Some(text) = filter.text_contains.as_deref() {
        let needle = text.trim().to_ascii_lowercase();
        if !needle.is_empty() {
            let summary = node
                .context_summary
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let raw = node.raw.to_ascii_lowercase();
            let session = node.session_id.to_ascii_lowercase();
            if !(summary.contains(&needle) || raw.contains(&needle) || session.contains(&needle)) {
                return false;
            }
        }
    }

    true
}
