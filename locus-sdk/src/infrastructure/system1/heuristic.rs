//! Offline System 1 stand-in for the memory reflex catalog.
//!
//! Cue counts are not a calibrated checkpoint. They exist so the gate, the
//! envelope, and tests have a decider that speaks the same five questions.
//! Attach [`super::HttpSystem1`] or call `MemoryReflexService::apply` when a
//! real Laya forward pass is available.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use async_trait::async_trait;
use serde_json::Value;

use crate::domain::reflex::{MemoryAction, SALIENCE_RUBRIC};
use crate::domain::system1::{
    DecisionAnswer, DecisionQuestion, System1Decider, System1Request, System1Response,
};

const RECALL: &[&str] = &[
    "do you remember",
    "remember when",
    "last time",
    "you said",
    "what did i",
    "what did we",
    "we discussed",
    "earlier you",
    "from before",
    "previous time",
    "as i mentioned",
];

const PERSIST: &[&str] = &[
    "remember that",
    "please remember",
    "note that",
    "my name is",
    "i prefer",
    "from now on",
    "don't forget",
    "do not forget",
    "save this",
    "i always",
    "i never",
    "keep in mind",
];

const FIND: &[&str] = &[
    "find the",
    "list the",
    "show me",
    "look up",
    "search for",
    "tagged",
    "filter by",
];

const AGGREGATE: &[&str] = &[
    "summarize",
    "summary of",
    "how many",
    "rollup",
    "over the last",
    "this week",
    "count the",
];

const EXPLAIN: &[&str] = &[
    "why did you recall",
    "why did you remember",
    "how did you know",
    "which memory",
    "explain the retrieval",
    "why that memory",
];

const ACKS: &[&str] = &[
    "ok",
    "okay",
    "k",
    "thanks",
    "thank you",
    "thx",
    "lol",
    "yes",
    "no",
    "yep",
    "nope",
    "sure",
    "cool",
    "got it",
    "hi",
    "hello",
    "hey",
];

struct Signals {
    action: MemoryAction,
    confidence: f32,
    salience_score: f32,
    references_prior: f32,
    should_persist: f32,
    needs_system2: f32,
}

/// Lexical decider for the five memory-reflex questions.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicSystem1;

#[async_trait]
impl System1Decider for HeuristicSystem1 {
    fn decider_id(&self) -> &str {
        "heuristic"
    }

    async fn predict(&self, request: &System1Request) -> Result<System1Response> {
        let signals = assess(&text_of(&request.state));
        let mut answers = BTreeMap::new();
        for (name, question) in &request.questions {
            answers.insert(name.clone(), answer_for(name, question, &signals)?);
        }
        Ok(System1Response {
            decider_id: self.decider_id().to_string(),
            checkpoint: Some("heuristic".to_string()),
            answers,
        })
    }
}

fn answer_for(
    name: &str,
    question: &DecisionQuestion,
    signals: &Signals,
) -> Result<DecisionAnswer> {
    match (name, question) {
        ("action", DecisionQuestion::Choice { .. }) => Ok(DecisionAnswer::Choice {
            choice: signals.action.as_str().to_string(),
            confidence: signals.confidence,
            probabilities: BTreeMap::from([(
                signals.action.as_str().to_string(),
                signals.confidence,
            )]),
        }),
        ("salience", DecisionQuestion::Score { .. }) => Ok(DecisionAnswer::Score {
            score: signals.salience_score,
            max: Some((SALIENCE_RUBRIC.len() - 1) as f32),
            label: Some(rubric_label(signals.salience_score)),
            confidence: signals.confidence,
        }),
        ("references_prior", DecisionQuestion::Noul { .. }) => Ok(DecisionAnswer::Noul {
            probability: signals.references_prior,
        }),
        ("should_persist", DecisionQuestion::Noul { .. }) => Ok(DecisionAnswer::Noul {
            probability: signals.should_persist,
        }),
        ("needs_system2", DecisionQuestion::Noul { .. }) => Ok(DecisionAnswer::Noul {
            probability: signals.needs_system2,
        }),
        _ => bail!("heuristic system 1 only answers the memory reflex catalog, not `{name}`"),
    }
}

fn assess(text: &str) -> Signals {
    let normalized = text.trim().to_lowercase();
    if is_ack(&normalized) {
        return Signals {
            action: MemoryAction::Ignore,
            confidence: 0.93,
            salience_score: 0.2,
            references_prior: 0.05,
            should_persist: 0.04,
            needs_system2: 0.08,
        };
    }

    let recall_hits = hits(&normalized, RECALL);
    let persist_hits = hits(&normalized, PERSIST);
    let find_hits = hits(&normalized, FIND);
    let aggregate_hits = hits(&normalized, AGGREGATE);
    let explain_hits = hits(&normalized, EXPLAIN);
    let read_hits = recall_hits + find_hits + aggregate_hits + explain_hits;

    if read_hits + persist_hits == 0 {
        let long = normalized.chars().count() > 24;
        return if long {
            Signals {
                action: MemoryAction::Ignore,
                confidence: 0.4,
                salience_score: 1.3,
                references_prior: 0.12,
                should_persist: 0.1,
                needs_system2: 0.86,
            }
        } else {
            Signals {
                action: MemoryAction::Ignore,
                confidence: 0.84,
                salience_score: 0.2,
                references_prior: 0.08,
                should_persist: 0.06,
                needs_system2: 0.1,
            }
        };
    }

    // Earlier entries win ties, so a more specific cue beats a general one.
    let scored = [
        (MemoryAction::Explain, explain_hits as f32),
        (MemoryAction::Persist, persist_hits as f32),
        (MemoryAction::Aggregate, aggregate_hits as f32),
        (MemoryAction::Find, find_hits as f32),
        (MemoryAction::Recall, recall_hits as f32),
        (MemoryAction::Ignore, 0.0),
    ];
    let best = scored
        .iter()
        .map(|(_, score)| *score)
        .fold(0.0_f32, f32::max);
    let action = scored
        .iter()
        .find(|(_, score)| (*score - best).abs() < f32::EPSILON)
        .map(|(action, _)| *action)
        .unwrap_or(MemoryAction::Ignore);
    let second = scored
        .iter()
        .filter(|(candidate, _)| *candidate != action)
        .map(|(_, score)| *score)
        .fold(0.0_f32, f32::max);
    let margin = (best - second).max(0.0);
    let mut confidence = (0.52 + margin * 0.2).clamp(0.52, 0.96);
    let mut needs_system2: f32 = if margin < 0.5 && second > 0.0 {
        0.74
    } else {
        0.1
    };
    if read_hits > 0 && persist_hits > 0 {
        needs_system2 = needs_system2.max(0.84);
        confidence = confidence.min(0.48);
    }
    let salience_score = if best >= 2.5 { 2.8 } else { 2.2 };

    Signals {
        action,
        confidence,
        salience_score,
        references_prior: probability_from_hits(read_hits, 0.72, 0.1),
        should_persist: probability_from_hits(persist_hits, 0.78, 0.08),
        needs_system2,
    }
}

fn probability_from_hits(hits: usize, base: f32, step: f32) -> f32 {
    if hits == 0 {
        0.08
    } else {
        (base + (hits - 1) as f32 * step).clamp(0.0, 0.97)
    }
}

fn hits(text: &str, phrases: &[&str]) -> usize {
    phrases
        .iter()
        .filter(|phrase| text.contains(*phrase))
        .count()
}

fn is_ack(normalized: &str) -> bool {
    let key = normalized
        .trim_matches(|c: char| !c.is_alphanumeric() && !c.is_whitespace())
        .trim();
    ACKS.contains(&key)
}

fn rubric_label(score: f32) -> String {
    let max = (SALIENCE_RUBRIC.len() - 1) as f32;
    let idx = (score.round().clamp(0.0, max)) as usize;
    SALIENCE_RUBRIC[idx.min(SALIENCE_RUBRIC.len() - 1)].to_string()
}

fn text_of(state: &Value) -> String {
    match state {
        Value::String(text) => text.clone(),
        Value::Object(map) => ["text", "body", "message"]
            .iter()
            .find_map(|key| map.get(*key).and_then(Value::as_str))
            .unwrap_or("")
            .to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::assess;
    use crate::domain::reflex::MemoryAction;

    #[test]
    fn acknowledgement_stays_quiet() {
        let signals = assess("Thanks!");
        assert_eq!(signals.action, MemoryAction::Ignore);
        assert!(signals.confidence > 0.9);
        assert!(signals.needs_system2 < 0.2);
    }

    #[test]
    fn recall_and_persist_cues_do_not_share_bare_remember() {
        let recall = assess("do you remember what we discussed about refunds");
        assert_eq!(recall.action, MemoryAction::Recall);
        assert!(recall.references_prior > 0.7);
        assert!(recall.should_persist < 0.2);

        let persist = assess("please remember that I prefer aisle seats");
        assert_eq!(persist.action, MemoryAction::Persist);
        assert!(persist.should_persist > 0.8);
        assert!(persist.references_prior < 0.2);
    }

    #[test]
    fn mixed_read_and_write_cues_ask_for_system2() {
        let signals = assess("please remember that, and do you remember what we discussed");
        assert!(signals.needs_system2 > 0.8);
        assert!(signals.confidence < 0.55);
    }
}
