use locus_core_rs::domain::models::SttpNode;

use crate::domain::memory::{FallbackPolicy, StrictnessMode};

/// Newest-first window scanned for natural-language term overlap.
///
/// This stays on the existing node query API. It does not add a text index.
pub const LEXICAL_SCAN_LIMIT: usize = 2000;

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "so", "of", "to", "for", "in", "on", "at",
    "from", "with", "by", "as", "into", "over", "under", "about", "what", "which", "who", "whom",
    "whose", "when", "where", "why", "how", "did", "do", "does", "is", "are", "was", "were", "be",
    "been", "being", "am", "we", "i", "you", "he", "she", "they", "it", "me", "my", "our", "your",
    "their", "them", "us", "this", "that", "these", "those", "there", "here", "please", "tell",
    "just", "any", "some", "not", "remember", "recall", "know",
];

/// How `query_text` should affect an already-ranked recall set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalActivation {
    /// Leave the primary ranking untouched.
    Skip,
    /// Exact phrase match, used when fallback runs for a single token.
    Legacy,
    /// Content-term overlap for a multi-word question or phrase.
    NaturalLanguage,
}

/// One content token plus the pieces of a hyphenated or underscored form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalQuery {
    pub phrase: String,
    pub coverage: Vec<Vec<String>>,
}

impl LexicalQuery {
    pub fn term_count(&self) -> usize {
        self.coverage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.phrase.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexicalFields {
    pub summary: bool,
    pub tags: bool,
    pub raw: bool,
    pub session: bool,
}

impl LexicalFields {
    pub const RECALL: Self = Self {
        summary: true,
        tags: true,
        raw: true,
        session: true,
    };

    pub const INVENTORY: Self = Self {
        summary: true,
        tags: true,
        raw: false,
        session: true,
    };
}

pub fn parse_lexical_query(query_text: &str) -> LexicalQuery {
    let phrase = query_text.trim().to_ascii_lowercase();
    let mut coverage = Vec::new();

    for raw in phrase.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
        let token = raw.trim_matches(|c: char| c == '-' || c == '_').to_string();
        if token.len() < 2 || is_stopword(&token) {
            continue;
        }

        let mut variants = vec![token.clone()];
        let parts = token
            .split(|c: char| c == '-' || c == '_')
            .filter(|part| part.len() >= 2 && !is_stopword(part))
            .map(str::to_string)
            .collect::<Vec<_>>();
        if parts.len() > 1 {
            for part in parts {
                if !variants.iter().any(|existing| existing == &part) {
                    variants.push(part);
                }
            }
        }

        if coverage.iter().any(|group: &Vec<String>| group[0] == token) {
            continue;
        }
        coverage.push(variants);
    }

    LexicalQuery { phrase, coverage }
}

pub fn activation(
    policy: FallbackPolicy,
    query_text: &str,
    primary_empty: bool,
) -> LexicalActivation {
    let query = parse_lexical_query(query_text);
    let natural = query.term_count() >= 2;

    match policy {
        FallbackPolicy::Never => LexicalActivation::Skip,
        FallbackPolicy::OnEmpty if natural => LexicalActivation::NaturalLanguage,
        FallbackPolicy::OnEmpty if primary_empty && !query.phrase.is_empty() => {
            LexicalActivation::Legacy
        }
        FallbackPolicy::OnEmpty => LexicalActivation::Skip,
        FallbackPolicy::Always if natural => LexicalActivation::NaturalLanguage,
        FallbackPolicy::Always if !query.phrase.is_empty() => LexicalActivation::Legacy,
        FallbackPolicy::Always => LexicalActivation::Skip,
    }
}

pub fn required_term_hits(term_count: usize, strictness: StrictnessMode) -> usize {
    if term_count == 0 {
        return 1;
    }

    match strictness {
        StrictnessMode::Precision => term_count,
        StrictnessMode::Balanced => term_count.div_ceil(2),
        StrictnessMode::Recall => 1,
    }
}

pub fn select_lexical_matches(
    nodes: Vec<SttpNode>,
    query: &LexicalQuery,
    strictness: StrictnessMode,
    fields: LexicalFields,
) -> Vec<SttpNode> {
    if query.term_count() == 0 {
        return Vec::new();
    }

    let required = required_term_hits(query.term_count(), strictness);
    let mut scored = nodes
        .into_iter()
        .filter_map(|node| {
            let (score, hits) = score_node(&node, query, fields);
            if hits >= required && score > 0 {
                Some((score, node.timestamp, node))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    scored.into_iter().map(|(_, _, node)| node).collect()
}

pub fn legacy_phrase_filter(nodes: Vec<SttpNode>, query_text: &str) -> Vec<SttpNode> {
    let needle = query_text.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return nodes;
    }

    let mut scored = nodes
        .into_iter()
        .filter_map(|node| {
            let summary = node
                .context_summary
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let session = node.session_id.to_ascii_lowercase();
            let raw = node.raw.to_ascii_lowercase();

            let mut score = 0usize;
            if summary.contains(&needle) {
                score += 3;
            }
            if session.contains(&needle) {
                score += 2;
            }
            if raw.contains(&needle) {
                score += 1;
            }

            if score > 0 {
                Some((score, node.timestamp, node))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    scored.into_iter().map(|(_, _, node)| node).collect()
}

pub fn merge_unique(primary: Vec<SttpNode>, secondary: Vec<SttpNode>) -> Vec<SttpNode> {
    let mut merged = Vec::with_capacity(primary.len() + secondary.len());
    let mut seen = std::collections::HashSet::new();

    for node in primary.into_iter().chain(secondary.into_iter()) {
        if seen.insert(node.sync_key.clone()) {
            merged.push(node);
        }
    }

    merged
}

/// Place lexical hits ahead of the primary ranking.
///
/// Without a query embedding, lexical hits replace resonance order.
/// With an embedding, hits are prepended and the hybrid tail is kept.
pub fn apply_natural_language(
    primary: Vec<SttpNode>,
    lexical_matches: Vec<SttpNode>,
    has_query_embedding: bool,
) -> (Vec<SttpNode>, bool) {
    if lexical_matches.is_empty() {
        return (primary, false);
    }

    if has_query_embedding {
        (merge_unique(lexical_matches, primary), true)
    } else {
        (lexical_matches, true)
    }
}

fn score_node(node: &SttpNode, query: &LexicalQuery, fields: LexicalFields) -> (u32, usize) {
    let summary = if fields.summary {
        node.context_summary
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    } else {
        String::new()
    };
    let raw = if fields.raw {
        node.raw.to_ascii_lowercase()
    } else {
        String::new()
    };
    let session = if fields.session {
        node.session_id.to_ascii_lowercase()
    } else {
        String::new()
    };
    let tags = if fields.tags {
        node.semantic_tags
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|tag| tag.to_ascii_lowercase())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut score = 0u32;
    let mut hits = 0usize;

    for variants in &query.coverage {
        let mut best = 0u32;
        for variant in variants {
            best = best.max(variant_score(
                variant, &summary, &raw, &session, &tags, fields,
            ));
        }
        if best > 0 {
            hits += 1;
            score += best;
        }
    }

    if query.phrase.contains(char::is_whitespace) {
        if fields.summary && summary.contains(&query.phrase) {
            score += 8;
        }
        if fields.raw && raw.contains(&query.phrase) {
            score += 4;
        }
    }

    (score, hits)
}

fn variant_score(
    term: &str,
    summary: &str,
    raw: &str,
    session: &str,
    tags: &[String],
    fields: LexicalFields,
) -> u32 {
    let mut score = 0u32;

    if fields.summary && field_contains_term(summary, term) {
        score += 3;
    }
    if fields.tags {
        if tags.iter().any(|tag| tag == term) {
            score += 5;
        } else if tags.iter().any(|tag| field_contains_term(tag, term)) {
            score += 3;
        }
    }
    if fields.raw && term.len() >= 4 && field_contains_term(raw, term) {
        score += 1;
    }
    if fields.session && term.len() >= 4 && field_contains_term(session, term) {
        score += 2;
    }

    score
}

fn field_contains_term(field: &str, term: &str) -> bool {
    if term.is_empty() {
        return false;
    }
    if field.contains(term) {
        return true;
    }
    if term.len() < 5 {
        return false;
    }

    field.split(|c: char| !c.is_alphanumeric()).any(|word| {
        if word.len() < 5 {
            return false;
        }
        word.starts_with(term) || term.starts_with(word)
    })
}

fn is_stopword(token: &str) -> bool {
    STOPWORDS.contains(&token)
}

#[cfg(test)]
mod tests {
    use super::{
        LexicalFields, activation, parse_lexical_query, required_term_hits, select_lexical_matches,
    };
    use crate::domain::memory::{FallbackPolicy, StrictnessMode};
    use chrono::Utc;
    use locus_core_rs::domain::models::{AvecState, SttpNode};

    #[test]
    fn question_drops_stopwords_and_keeps_content_terms() {
        let query = parse_lexical_query("what did we decide about the parser grammar?");
        let surface = query
            .coverage
            .iter()
            .map(|variants| variants[0].as_str())
            .collect::<Vec<_>>();
        assert_eq!(surface, vec!["decide", "parser", "grammar"]);
    }

    #[test]
    fn hyphenated_token_keeps_compound_and_parts() {
        let query = parse_lexical_query("strict-mode");
        assert_eq!(
            query.coverage,
            vec![vec![
                "strict-mode".to_string(),
                "strict".to_string(),
                "mode".to_string()
            ]]
        );
    }

    #[test]
    fn natural_language_activates_even_when_primary_is_non_empty() {
        assert_eq!(
            activation(
                FallbackPolicy::OnEmpty,
                "what did we decide about parser grammar",
                false
            ),
            super::LexicalActivation::NaturalLanguage
        );
        assert_eq!(
            activation(FallbackPolicy::OnEmpty, "parser", false),
            super::LexicalActivation::Skip
        );
        assert_eq!(
            activation(
                FallbackPolicy::Never,
                "what did we decide about parser grammar",
                true
            ),
            super::LexicalActivation::Skip
        );
    }

    #[test]
    fn inflection_and_phrase_bonus_rank_the_closer_summary_first() {
        let query = parse_lexical_query("what did we decide about the parser grammar");
        let decided = node(
            "decided",
            "decided to harden the parser grammar",
            "notes",
            None,
        );
        let partial = node(
            "partial",
            "parser grammar notes from standup",
            "other",
            None,
        );
        let ranked = select_lexical_matches(
            vec![partial, decided],
            &query,
            StrictnessMode::Balanced,
            LexicalFields::RECALL,
        );

        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].sync_key, "decided");
    }

    #[test]
    fn precision_requires_every_content_term() {
        let query = parse_lexical_query("parser grammar rollout");
        assert_eq!(
            required_term_hits(query.term_count(), StrictnessMode::Precision),
            3
        );
        let partial = node("partial", "parser notes", "parser", None);
        let ranked = select_lexical_matches(
            vec![partial],
            &query,
            StrictnessMode::Precision,
            LexicalFields::RECALL,
        );
        assert!(ranked.is_empty());
    }

    fn node(sync_key: &str, summary: &str, raw: &str, tags: Option<Vec<&str>>) -> SttpNode {
        let now = Utc::now();
        let avec = AvecState::zero();
        SttpNode {
            raw: raw.to_string(),
            session_id: "session".to_string(),
            tier: "raw".to_string(),
            timestamp: now,
            compression_depth: 1,
            parent_node_id: None,
            sync_key: sync_key.to_string(),
            updated_at: now,
            source_metadata: None,
            context_summary: Some(summary.to_string()),
            semantic_tags: tags.map(|values| values.into_iter().map(str::to_string).collect()),
            semantic_links: None,
            embedding_dimensions: None,
            embedding_model: None,
            embedding: None,
            embedded_at: None,
            user_avec: avec,
            model_avec: avec,
            compression_avec: Some(avec),
            rho: 0.5,
            kappa: 0.5,
            psi: 1.0,
        }
    }
}
