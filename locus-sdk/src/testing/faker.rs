use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::{Duration, Utc};
use fake::faker::lorem::en::Sentence;
use fake::rand::SeedableRng as FakeSeedableRng;
use fake::rand::rngs::StdRng as FakeStdRng;
use fake::Fake;
use rand::distributions::{Distribution, WeightedIndex};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightedTerm {
    pub term: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierWeights {
    pub raw: u32,
    pub daily: u32,
    pub weekly: u32,
    pub monthly: u32,
}

impl Default for TierWeights {
    fn default() -> Self {
        Self {
            raw: 70,
            daily: 15,
            weekly: 10,
            monthly: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FakerConfig {
    pub seed: u64,
    pub sessions: usize,
    pub min_nodes_per_session: usize,
    pub max_nodes_per_session: usize,
    pub tier_distribution: TierWeights,
    pub filler_ratio: f32,
    pub topic_drift: f32,
    pub timestamp_span_days: usize,
    pub domain_lexicon: Vec<WeightedTerm>,
}

impl Default for FakerConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            sessions: 5,
            min_nodes_per_session: 5,
            max_nodes_per_session: 15,
            tier_distribution: TierWeights::default(),
            filler_ratio: 0.18,
            topic_drift: 0.22,
            timestamp_span_days: 30,
            domain_lexicon: vec![
                WeightedTerm {
                    term: "retrieval".to_string(),
                    weight: 10,
                },
                WeightedTerm {
                    term: "session".to_string(),
                    weight: 10,
                },
                WeightedTerm {
                    term: "embedding".to_string(),
                    weight: 9,
                },
                WeightedTerm {
                    term: "fallback".to_string(),
                    weight: 8,
                },
                WeightedTerm {
                    term: "aggregate".to_string(),
                    weight: 7,
                },
                WeightedTerm {
                    term: "transform".to_string(),
                    weight: 7,
                },
                WeightedTerm {
                    term: "schema".to_string(),
                    weight: 6,
                },
                WeightedTerm {
                    term: "parser".to_string(),
                    weight: 5,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseProfile {
    pub filler_ratio_actual: f32,
    pub distractor_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FakerOutputRecord {
    pub synthetic_id: String,
    pub session_id: String,
    pub tier: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub raw_text: String,
    pub expected_anchor_terms: Vec<String>,
    pub noise_profile: NoiseProfile,
}

pub struct SttpFakerBuilder {
    config: FakerConfig,
}

impl SttpFakerBuilder {
    pub fn new(config: FakerConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self) -> Vec<FakerOutputRecord> {
        let mut rng = ChaCha8Rng::seed_from_u64(self.config.seed);
        let mut fake_rng = FakeStdRng::seed_from_u64(self.config.seed);

        let min_nodes = self.config.min_nodes_per_session.max(1);
        let max_nodes = self.config.max_nodes_per_session.max(min_nodes);
        let span_days = self.config.timestamp_span_days.max(1) as i64;

        let mut records = Vec::new();

        for session_index in 0..self.config.sessions.max(1) {
            let session_id = format!("session-{:03}", session_index + 1);
            let node_count = rng.gen_range(min_nodes..=max_nodes);

            for node_index in 0..node_count {
                let tier = sample_tier(&self.config.tier_distribution, &mut rng);
                let day_offset = rng.gen_range(0..span_days);
                let minute_offset = rng.gen_range(0..(24 * 60));
                let timestamp = Utc::now()
                    - Duration::days(day_offset)
                    - Duration::minutes(minute_offset);

                let anchor_terms = sample_anchor_terms(&self.config.domain_lexicon, 5, &mut rng);
                let (raw_text, noise_profile) = compose_text(
                    &anchor_terms,
                    self.config.filler_ratio,
                    self.config.topic_drift,
                    &mut rng,
                    &mut fake_rng,
                );

                records.push(FakerOutputRecord {
                    synthetic_id: format!("{}-{:04}", session_id, node_index + 1),
                    session_id: session_id.clone(),
                    tier,
                    timestamp,
                    raw_text,
                    expected_anchor_terms: anchor_terms,
                    noise_profile,
                });
            }
        }

        records
    }
}

pub fn records_to_jsonl(records: &[FakerOutputRecord]) -> Result<String> {
    let lines = records
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(lines.join("\n"))
}

pub fn write_jsonl_fixture(path: &Path, records: &[FakerOutputRecord]) -> Result<()> {
    let jsonl = records_to_jsonl(records)?;
    fs::write(path, jsonl)?;
    Ok(())
}

fn sample_tier(weights: &TierWeights, rng: &mut ChaCha8Rng) -> String {
    let labels = ["raw", "daily", "weekly", "monthly"];
    let weight_values = [weights.raw, weights.daily, weights.weekly, weights.monthly];

    let index = if let Ok(dist) = WeightedIndex::new(weight_values) {
        dist.sample(rng)
    } else {
        0
    };

    labels[index].to_string()
}

fn sample_anchor_terms(lexicon: &[WeightedTerm], count: usize, rng: &mut ChaCha8Rng) -> Vec<String> {
    if lexicon.is_empty() {
        return vec!["memory".to_string(), "session".to_string()];
    }

    let mut picked = Vec::new();
    let weights = lexicon
        .iter()
        .map(|item| item.weight.max(1))
        .collect::<Vec<_>>();

    for _ in 0..count.max(1) {
        let index = if let Ok(dist) = WeightedIndex::new(&weights) {
            dist.sample(rng)
        } else {
            rng.gen_range(0..lexicon.len())
        };
        let term = lexicon[index].term.clone();
        if !picked.iter().any(|existing| existing == &term) {
            picked.push(term);
        }
    }

    if picked.is_empty() {
        picked.push(lexicon[0].term.clone());
    }

    picked
}

fn compose_text(
    anchors: &[String],
    filler_ratio: f32,
    topic_drift: f32,
    rng: &mut ChaCha8Rng,
    fake_rng: &mut FakeStdRng,
) -> (String, NoiseProfile) {
    let filler_words = [
        "basically",
        "actually",
        "really",
        "just",
        "kind of",
        "sort of",
        "you know",
        "i mean",
    ];

    let mut fragments = Vec::new();
    for anchor in anchors {
        let phrase = format!("{} pipeline stability and retrieval behavior", anchor);
        fragments.push(phrase);
    }

    let extra_sentence: String = Sentence(6..12).fake_with_rng(fake_rng);
    fragments.push(extra_sentence.to_ascii_lowercase());

    let mut filler_count = 0usize;
    let mut distractor_count = 0usize;
    let sample_size = fragments.len().max(1);

    for _ in 0..sample_size {
        if rng.gen_bool(filler_ratio.clamp(0.0, 1.0) as f64) {
            if let Some(filler) = filler_words.choose(rng) {
                fragments.push((*filler).to_string());
                filler_count += 1;
            }
        }

        if rng.gen_bool(topic_drift.clamp(0.0, 1.0) as f64) {
            let distractor: String = Sentence(3..8).fake_with_rng(fake_rng);
            fragments.push(distractor.to_ascii_lowercase());
            distractor_count += 1;
        }
    }

    fragments.shuffle(rng);

    let total = fragments.len().max(1) as f32;
    let noise_profile = NoiseProfile {
        filler_ratio_actual: filler_count as f32 / total,
        distractor_ratio: distractor_count as f32 / total,
    };

    (fragments.join(". "), noise_profile)
}

#[cfg(test)]
mod tests {
    use super::{FakerConfig, SttpFakerBuilder, records_to_jsonl};

    #[test]
    fn seeded_generation_is_deterministic() {
        let config = FakerConfig {
            seed: 7,
            sessions: 2,
            min_nodes_per_session: 2,
            max_nodes_per_session: 2,
            ..Default::default()
        };

        let left = SttpFakerBuilder::new(config.clone()).generate();
        let right = SttpFakerBuilder::new(config).generate();

        assert_eq!(left.len(), right.len());
        assert_eq!(left[0].raw_text, right[0].raw_text);
        assert_eq!(left[0].expected_anchor_terms, right[0].expected_anchor_terms);
    }

    #[test]
    fn jsonl_export_produces_one_line_per_record() {
        let config = FakerConfig {
            seed: 9,
            sessions: 1,
            min_nodes_per_session: 3,
            max_nodes_per_session: 3,
            ..Default::default()
        };

        let records = SttpFakerBuilder::new(config).generate();
        let jsonl = records_to_jsonl(&records).expect("jsonl export should succeed");

        assert_eq!(jsonl.lines().count(), 3);
    }
}
