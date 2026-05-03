use std::collections::{HashMap, HashSet};

use crate::domain::compression::{
    AnchorTerm, ManualCompressionDiagnostics, ManualCompressionRequest, ManualCompressionResult,
    PhraseMode, StopwordProfile,
};

pub struct CompressionLexicons {
    pub stopwords: HashSet<String>,
    pub fillers: HashSet<String>,
    pub negations: HashSet<String>,
}

pub trait ManualCompressionLexiconProvider: Send + Sync {
    fn build_lexicons(&self, request: &ManualCompressionRequest) -> CompressionLexicons;
}

#[derive(Default)]
pub struct DefaultManualCompressionLexiconProvider;

impl ManualCompressionLexiconProvider for DefaultManualCompressionLexiconProvider {
    fn build_lexicons(&self, request: &ManualCompressionRequest) -> CompressionLexicons {
        let mut stopwords = stopword_set(request.stopword_profile);
        let mut fillers = filler_set();
        let mut negations = negation_set();
        apply_lexicon_overrides(request, &mut stopwords, &mut fillers, &mut negations);

        CompressionLexicons {
            stopwords,
            fillers,
            negations,
        }
    }
}

pub struct ManualCompressionService {
    lexicon_provider: Box<dyn ManualCompressionLexiconProvider>,
}

impl ManualCompressionService {
    pub fn new() -> Self {
        Self::with_lexicon_provider(DefaultManualCompressionLexiconProvider)
    }

    pub fn with_lexicon_provider(provider: impl ManualCompressionLexiconProvider + 'static) -> Self {
        Self {
            lexicon_provider: Box::new(provider),
        }
    }

    pub fn execute(&self, request: &ManualCompressionRequest) -> ManualCompressionResult {
        let sentences = split_sentences(&request.text);
        let sentence_count = sentences.len();

        let all_tokens = tokenize(&request.text);
        let lexicons = self.lexicon_provider.build_lexicons(request);
        let stopwords = &lexicons.stopwords;
        let filler = &lexicons.fillers;
        let negations = &lexicons.negations;

        let mut kept_tokens = Vec::new();

        let token_result = select_content_tokens(
            &all_tokens,
            &stopwords,
            &filler,
            &negations,
            request.min_token_length,
        );
        kept_tokens.extend(token_result.tokens);
        let stopwords_removed = token_result.stopwords_removed;
        let filler_removed = token_result.filler_removed;

        let term_stats = build_term_stats(&kept_tokens);
        let cooccur = sentence_cooccurrence(
            &sentences,
            &stopwords,
            &filler,
            &negations,
            request.min_token_length,
        );
        let anchors = score_anchors(&term_stats, &cooccur, request.max_anchors.max(1));

        let salient_phrases = match request.phrase_mode {
            PhraseMode::None => Vec::new(),
            PhraseMode::RakeLite => {
                rake_lite_phrases(&sentences, &anchors, &stopwords, &filler, &negations, request.min_token_length, 5)
            }
        };

        let anchor_topic = choose_anchor_topic(&anchors, &salient_phrases);
        let key_points = build_key_points(
            &sentences,
            &anchors,
            &stopwords,
            &filler,
            &negations,
            request.min_token_length,
            request.max_points.max(1),
        );

        let input_tokens = all_tokens.len().max(1) as f32;
        let output_tokens = (tokenize(&anchor_topic).len()
            + anchors.len()
            + salient_phrases.iter().map(|p| tokenize(p).len()).sum::<usize>()
            + key_points.iter().map(|p| tokenize(p).len()).sum::<usize>()) as f32;

        let diagnostics = ManualCompressionDiagnostics {
            tokens_total: all_tokens.len(),
            tokens_kept: kept_tokens.len(),
            stopwords_removed,
            filler_removed,
            sentences_total: sentence_count,
        };

        ManualCompressionResult {
            anchor_topic,
            anchor_terms: anchors,
            key_points,
            salient_phrases,
            compression_ratio: (output_tokens / input_tokens).clamp(0.0, 10.0),
            discarded_noise_ratio: ((stopwords_removed + filler_removed) as f32 / input_tokens)
                .clamp(0.0, 1.0),
            diagnostics,
        }
    }
}

impl Default for ManualCompressionService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct TermStat {
    freq: usize,
    first_pos: usize,
}

fn split_sentences(text: &str) -> Vec<String> {
    text.split(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    let normalized = text
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch } else { ' ' })
        .collect::<String>();

    normalized
        .split_whitespace()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .collect()
}

fn normalize_lexicon_entry(value: &str) -> Option<String> {
    let normalized = tokenize(value);
    normalized.first().cloned()
}

fn apply_lexicon_overrides(
    request: &ManualCompressionRequest,
    stopwords: &mut HashSet<String>,
    fillers: &mut HashSet<String>,
    negations: &mut HashSet<String>,
) {
    for value in &request.stopwords_add {
        if let Some(entry) = normalize_lexicon_entry(value) {
            stopwords.insert(entry);
        }
    }
    for value in &request.stopwords_remove {
        if let Some(entry) = normalize_lexicon_entry(value) {
            stopwords.remove(&entry);
        }
    }

    for value in &request.fillers_add {
        if let Some(entry) = normalize_lexicon_entry(value) {
            fillers.insert(entry);
        }
    }
    for value in &request.fillers_remove {
        if let Some(entry) = normalize_lexicon_entry(value) {
            fillers.remove(&entry);
        }
    }

    for value in &request.negations_add {
        if let Some(entry) = normalize_lexicon_entry(value) {
            negations.insert(entry);
        }
    }
    for value in &request.negations_remove {
        if let Some(entry) = normalize_lexicon_entry(value) {
            negations.remove(&entry);
        }
    }
}

#[derive(Default)]
struct ContentTokenSelection {
    tokens: Vec<String>,
    stopwords_removed: usize,
    filler_removed: usize,
}

fn is_negated_marker(token: &str) -> bool {
    token.starts_with("neg_")
}

fn select_content_tokens(
    raw_tokens: &[String],
    stopwords: &HashSet<String>,
    filler: &HashSet<String>,
    negations: &HashSet<String>,
    min_len: usize,
) -> ContentTokenSelection {
    let mut selected = ContentTokenSelection::default();
    let mut pending_negation = false;

    for token in raw_tokens {
        if negations.contains(token.as_str()) {
            // Toggle polarity so repeated negations cancel each other.
            pending_negation = !pending_negation;
            continue;
        }

        if filler.contains(token.as_str()) {
            selected.filler_removed += 1;
            continue;
        }

        if stopwords.contains(token.as_str()) {
            selected.stopwords_removed += 1;
            continue;
        }

        if token.len() < min_len.max(1) {
            continue;
        }

        if pending_negation {
            selected.tokens.push(format!("neg_{token}"));
            pending_negation = false;
        } else {
            selected.tokens.push(token.clone());
        }
    }

    selected
}

fn stopword_set(profile: StopwordProfile) -> HashSet<String> {
    let basic = [
        "the", "a", "an", "and", "or", "for", "to", "of", "in", "on", "at", "is", "are",
        "was", "were", "be", "been", "with", "that", "this", "it", "as", "by", "from", "we",
    ];

    let extended = [
        "have", "has", "had", "do", "does", "did", "if", "then", "else", "when", "while", "into",
        "about", "over", "under", "very", "more", "most", "some", "any", "all", "each", "only",
    ];

    let domain = [
        "node", "nodes", "session", "sessions", "memory", "sttp", "context", "data", "value", "values",
    ];

    let mut set: HashSet<String> = basic.into_iter().map(str::to_string).collect();

    if matches!(profile, StopwordProfile::Extended | StopwordProfile::Domain) {
        set.extend(extended.into_iter().map(str::to_string));
    }
    if matches!(profile, StopwordProfile::Domain) {
        set.extend(domain.into_iter().map(str::to_string));
    }

    set
}

fn filler_set() -> HashSet<String> {
    [
        "basically",
        "actually",
        "really",
        "just",
        "literally",
        "kinda",
        "sorta",
        "maybe",
        "honestly",
        "frankly",
        "perhaps",
        "probably",
        "simply",
        "quite",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn negation_set() -> HashSet<String> {
    ["not", "no", "never", "none", "cannot", "neither", "nor"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn build_term_stats(tokens: &[String]) -> HashMap<String, TermStat> {
    let mut map: HashMap<String, TermStat> = HashMap::new();

    for (idx, token) in tokens.iter().enumerate() {
        map.entry(token.clone())
            .and_modify(|stat| stat.freq += 1)
            .or_insert(TermStat {
                freq: 1,
                first_pos: idx,
            });
    }

    map
}

fn sentence_cooccurrence(
    sentences: &[String],
    stopwords: &HashSet<String>,
    filler: &HashSet<String>,
    negations: &HashSet<String>,
    min_len: usize,
) -> HashMap<String, usize> {
    let mut degree = HashMap::new();

    for sentence in sentences {
        let sentence_tokens = tokenize(sentence);
        let mut uniq = select_content_tokens(&sentence_tokens, stopwords, filler, negations, min_len)
            .tokens
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        uniq.sort();

        for token in uniq.iter() {
            let entry = degree.entry(token.clone()).or_insert(0usize);
            *entry += uniq.len().saturating_sub(1);
        }
    }

    degree
}

fn score_anchors(
    term_stats: &HashMap<String, TermStat>,
    cooccur: &HashMap<String, usize>,
    max_anchors: usize,
) -> Vec<AnchorTerm> {
    if term_stats.is_empty() {
        return Vec::new();
    }

    let max_freq = term_stats.values().map(|s| s.freq).max().unwrap_or(1) as f32;
    let max_pos = term_stats.values().map(|s| s.first_pos).max().unwrap_or(1) as f32;
    let max_degree = cooccur.values().copied().max().unwrap_or(1) as f32;

    let mut anchors = term_stats
        .iter()
        .map(|(term, stat)| {
            let freq_norm = stat.freq as f32 / max_freq;
            let pos_boost = 1.0 - (stat.first_pos as f32 / (max_pos + 1.0));
            let rarity_boost = 1.0 / stat.freq as f32;
            let centrality = *cooccur.get(term).unwrap_or(&0) as f32 / max_degree;

            let score = (0.45 * freq_norm) + (0.20 * pos_boost) + (0.20 * rarity_boost) + (0.15 * centrality);

            AnchorTerm {
                term: term.clone(),
                score,
                evidence_count: stat.freq,
                first_position: stat.first_pos,
            }
        })
        .collect::<Vec<_>>();

    anchors.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.evidence_count.cmp(&a.evidence_count))
            .then_with(|| a.term.cmp(&b.term))
    });

    anchors.truncate(max_anchors);
    anchors
}

fn rake_lite_phrases(
    sentences: &[String],
    anchors: &[AnchorTerm],
    stopwords: &HashSet<String>,
    filler: &HashSet<String>,
    negations: &HashSet<String>,
    min_len: usize,
    limit: usize,
) -> Vec<String> {
    if anchors.is_empty() {
        return Vec::new();
    }

    let anchor_set = anchors
        .iter()
        .map(|a| a.term.as_str())
        .collect::<HashSet<_>>();

    let mut scored = Vec::new();

    for sentence in sentences {
        let sentence_tokens = tokenize(sentence);
        let tokens = select_content_tokens(&sentence_tokens, stopwords, filler, negations, min_len).tokens;
        for window in [2usize, 3usize] {
            if tokens.len() < window {
                continue;
            }
            for idx in 0..=(tokens.len() - window) {
                let phrase = tokens[idx..idx + window].to_vec();
                let overlap = phrase.iter().filter(|t| anchor_set.contains(t.as_str())).count();
                if overlap > 0 {
                    let score = (overlap as f32)
                        + (window as f32 * 0.1)
                        + (phrase.iter().filter(|t| is_negated_marker(t)).count() as f32 * 0.15);
                    let phrase_text = phrase
                        .iter()
                        .map(|token| token.strip_prefix("neg_").map(|t| format!("not {t}")).unwrap_or_else(|| token.clone()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    scored.push((score, phrase_text));
                }
            }
        }
    }

    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });

    let mut seen = HashSet::new();
    scored
        .into_iter()
        .filter_map(|(_, phrase)| {
            if seen.insert(phrase.clone()) {
                Some(phrase)
            } else {
                None
            }
        })
        .take(limit)
        .collect()
}

fn choose_anchor_topic(anchors: &[AnchorTerm], phrases: &[String]) -> String {
    let Some(top) = anchors.first() else {
        return "".to_string();
    };

    let top_display = top.term.strip_prefix("neg_").map(|t| format!("not {t}")).unwrap_or_else(|| top.term.clone());

    if let Some(phrase) = phrases.iter().find(|phrase| phrase.contains(&top_display)) {
        return phrase.clone();
    }

    top_display
}

fn build_key_points(
    sentences: &[String],
    anchors: &[AnchorTerm],
    stopwords: &HashSet<String>,
    filler: &HashSet<String>,
    negations: &HashSet<String>,
    min_len: usize,
    max_points: usize,
) -> Vec<String> {
    if anchors.is_empty() {
        return sentences.iter().take(max_points).cloned().collect();
    }

    let anchor_terms = anchors
        .iter()
        .map(|anchor| anchor.term.as_str())
        .collect::<HashSet<_>>();

    let mut scored = sentences
        .iter()
        .enumerate()
        .map(|(idx, sentence)| {
            let sentence_tokens = tokenize(sentence);
            let tokens = select_content_tokens(&sentence_tokens, stopwords, filler, negations, min_len).tokens;
            let hits = tokens
                .iter()
                .filter(|token| anchor_terms.contains(token.as_str()))
                .count();
            let score = (hits as f32 * 2.0) + (1.0 / ((idx + 1) as f32));
            (score, idx, sentence.clone())
        })
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });

    scored
        .into_iter()
        .filter(|(score, _, _)| *score > 0.0)
        .take(max_points)
        .map(|(_, _, sentence)| sentence)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{CompressionLexicons, ManualCompressionLexiconProvider, ManualCompressionService};
    use crate::domain::compression::ManualCompressionRequest;

    struct CustomLexiconProvider;

    impl ManualCompressionLexiconProvider for CustomLexiconProvider {
        fn build_lexicons(&self, _request: &ManualCompressionRequest) -> CompressionLexicons {
            CompressionLexicons {
                stopwords: ["retrieval"].into_iter().map(str::to_string).collect::<HashSet<_>>(),
                fillers: ["fallback"].into_iter().map(str::to_string).collect::<HashSet<_>>(),
                negations: ["hardly"].into_iter().map(str::to_string).collect::<HashSet<_>>(),
            }
        }
    }

    #[test]
    fn compressor_filters_filler_and_extracts_anchor() {
        let service = ManualCompressionService::new();
        let request = ManualCompressionRequest {
            text: "Basically we need retrieval fallback policy. Actually retrieval fallback keeps recall stable.".to_string(),
            ..Default::default()
        };

        let result = service.execute(&request);

        assert!(result.anchor_terms.iter().any(|term| term.term == "retrieval" || term.term == "fallback"));
        assert!(!result.anchor_terms.iter().any(|term| term.term == "basically"));
        assert!(result.discarded_noise_ratio > 0.0);
    }

    #[test]
    fn compressor_is_deterministic_for_same_input() {
        let service = ManualCompressionService::new();
        let request = ManualCompressionRequest {
            text: "session graph recall and migration policy stability".to_string(),
            ..Default::default()
        };

        let left = service.execute(&request);
        let right = service.execute(&request);

        assert_eq!(left.anchor_topic, right.anchor_topic);
        assert_eq!(left.anchor_terms.len(), right.anchor_terms.len());
        assert_eq!(left.key_points, right.key_points);
    }

    #[test]
    fn compressor_cancels_double_negatives() {
        let service = ManualCompressionService::new();
        let request = ManualCompressionRequest {
            text: "the rollout is not not stable and the policy is not brittle".to_string(),
            ..Default::default()
        };

        let result = service.execute(&request);

        assert!(result.anchor_terms.iter().any(|term| term.term == "stable"));
        assert!(result.anchor_terms.iter().any(|term| term.term == "neg_brittle"));
        assert!(!result.anchor_terms.iter().any(|term| term.term == "neg_stable"));
    }

    #[test]
    fn compressor_filters_expanded_filler_lexicon() {
        let service = ManualCompressionService::new();
        let request = ManualCompressionRequest {
            text: "Honestly the migration policy is quite stable and honestly deterministic".to_string(),
            ..Default::default()
        };

        let result = service.execute(&request);

        assert!(!result.anchor_terms.iter().any(|term| term.term == "honestly" || term.term == "quite"));
        assert!(result.diagnostics.filler_removed >= 2);
    }

    #[test]
    fn compressor_honors_request_lexicon_overrides() {
        let service = ManualCompressionService::new();
        let request = ManualCompressionRequest {
            text: "hardly brittle retrieval retrieval fallback maybe".to_string(),
            stopwords_add: vec!["retrieval".to_string()],
            fillers_add: vec!["fallback".to_string()],
            negations_add: vec!["hardly".to_string()],
            ..Default::default()
        };

        let result = service.execute(&request);

        assert!(!result.anchor_terms.iter().any(|term| term.term == "retrieval"));
        assert!(!result.anchor_terms.iter().any(|term| term.term == "fallback"));
        assert!(result.anchor_terms.iter().any(|term| term.term == "neg_brittle"));
    }

    #[test]
    fn compressor_supports_custom_trait_provider() {
        let service = ManualCompressionService::with_lexicon_provider(CustomLexiconProvider);
        let request = ManualCompressionRequest {
            text: "hardly brittle retrieval retrieval fallback".to_string(),
            ..Default::default()
        };

        let result = service.execute(&request);

        assert!(!result.anchor_terms.iter().any(|term| term.term == "retrieval"));
        assert!(!result.anchor_terms.iter().any(|term| term.term == "fallback"));
        assert!(result.anchor_terms.iter().any(|term| term.term == "neg_brittle"));
    }
}
