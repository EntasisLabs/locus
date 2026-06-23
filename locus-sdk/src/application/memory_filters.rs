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

    if let Some(expected_tag) = filter.has_tag.as_deref() {
        let needle = expected_tag.trim().to_ascii_lowercase();
        if !needle.is_empty() {
            let tags = node.semantic_tags.as_deref().unwrap_or_default();
            if !tags
                .iter()
                .any(|tag| tag.to_ascii_lowercase() == needle)
            {
                return false;
            }
        }
    }

    if let Some(required_tags) = filter.tags_contains.as_ref() {
        let node_tags = node
            .semantic_tags
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect::<HashSet<_>>();

        if required_tags.iter().any(|tag| {
            let needle = tag.trim().to_ascii_lowercase();
            !needle.is_empty() && !node_tags.contains(&needle)
        }) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use locus_core_rs::domain::models::{AvecState, SttpNode};

    use super::node_matches_common_filters;
    use crate::domain::memory::{MemoryFilter, MemoryScope};

    fn sample_node(tags: Option<Vec<&str>>) -> SttpNode {
        SttpNode {
            raw: "raw".to_string(),
            session_id: "session-a".to_string(),
            tier: "raw".to_string(),
            timestamp: Utc::now(),
            compression_depth: 1,
            parent_node_id: None,
            sync_key: String::new(),
            updated_at: Utc::now(),
            source_metadata: None,
            context_summary: Some("summary".to_string()),
            semantic_tags: tags.map(|values| {
                values
                    .into_iter()
                    .map(|value| value.to_string())
                    .collect()
            }),
            semantic_links: None,
            embedding: None,
            embedding_model: None,
            embedding_dimensions: None,
            embedded_at: None,
            user_avec: AvecState::zero(),
            model_avec: AvecState::zero(),
            compression_avec: None,
            rho: 0.0,
            kappa: 0.0,
            psi: 0.0,
        }
    }

    #[test]
    fn has_tag_filter_matches_semantic_tags() {
        let node = sample_node(Some(vec!["safety-eval", "parser"]));
        let filter = MemoryFilter {
            has_tag: Some("parser".to_string()),
            ..Default::default()
        };

        assert!(node_matches_common_filters(
            &node,
            &MemoryScope::default(),
            &filter,
            None
        ));
    }

    #[test]
    fn tags_contains_requires_all_tags() {
        let node = sample_node(Some(vec!["safety-eval", "parser"]));
        let filter = MemoryFilter {
            tags_contains: Some(vec!["parser".to_string(), "missing".to_string()]),
            ..Default::default()
        };

        assert!(!node_matches_common_filters(
            &node,
            &MemoryScope::default(),
            &filter,
            None
        ));
    }
}
