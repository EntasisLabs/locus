use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopwordProfile {
    Basic,
    Extended,
    Domain,
}

impl Default for StopwordProfile {
    fn default() -> Self {
        Self::Domain
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PhraseMode {
    None,
    RakeLite,
}

impl Default for PhraseMode {
    fn default() -> Self {
        Self::RakeLite
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualCompressionRequest {
    pub text: String,
    pub max_anchors: usize,
    pub max_points: usize,
    pub min_token_length: usize,
    pub stopword_profile: StopwordProfile,
    pub phrase_mode: PhraseMode,
    #[serde(default)]
    pub stopwords_add: Vec<String>,
    #[serde(default)]
    pub stopwords_remove: Vec<String>,
    #[serde(default)]
    pub fillers_add: Vec<String>,
    #[serde(default)]
    pub fillers_remove: Vec<String>,
    #[serde(default)]
    pub negations_add: Vec<String>,
    #[serde(default)]
    pub negations_remove: Vec<String>,
}

impl Default for ManualCompressionRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            max_anchors: 5,
            max_points: 5,
            min_token_length: 3,
            stopword_profile: StopwordProfile::Domain,
            phrase_mode: PhraseMode::RakeLite,
            stopwords_add: Vec::new(),
            stopwords_remove: Vec::new(),
            fillers_add: Vec::new(),
            fillers_remove: Vec::new(),
            negations_add: Vec::new(),
            negations_remove: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorTerm {
    pub term: String,
    pub score: f32,
    pub evidence_count: usize,
    pub first_position: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualCompressionDiagnostics {
    pub tokens_total: usize,
    pub tokens_kept: usize,
    pub stopwords_removed: usize,
    pub filler_removed: usize,
    pub sentences_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualCompressionResult {
    pub anchor_topic: String,
    pub anchor_terms: Vec<AnchorTerm>,
    pub key_points: Vec<String>,
    pub salient_phrases: Vec<String>,
    pub compression_ratio: f32,
    pub discarded_noise_ratio: f32,
    pub diagnostics: ManualCompressionDiagnostics,
}
