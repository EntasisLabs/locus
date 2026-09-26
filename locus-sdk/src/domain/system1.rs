//! System 1 decision contract.
//!
//! A System 1 model answers typed `choice`, `score`, and `noul` questions about a
//! state in one forward pass. Laya, the `sys1` server, and Jev-compatible hosts
//! all speak this shape at `POST /v1/systemone`. This module is that contract.
//! It does not load weights and it does not open a socket.

use std::collections::BTreeMap;

use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Typed question a System 1 model can answer without generating text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecisionQuestion {
    /// Pick one label from `criteria` (label → description).
    Choice {
        instructions: String,
        criteria: BTreeMap<String, String>,
    },
    /// Place the state on an ordered rubric. Index 0 is the low end.
    Score {
        instructions: String,
        criteria: Vec<String>,
    },
    /// Probability that the instruction is true, in `0..=1`.
    Noul { instructions: String },
}

/// One answer from a System 1 forward pass.
#[derive(Debug, Clone, PartialEq)]
pub enum DecisionAnswer {
    Choice {
        choice: String,
        confidence: f32,
        probabilities: BTreeMap<String, f32>,
    },
    Score {
        score: f32,
        max: Option<f32>,
        label: Option<String>,
        confidence: f32,
    },
    Noul {
        probability: f32,
    },
}

/// Body a host can POST to a System 1 endpoint, or pass to an in-process decider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct System1Request {
    pub state: Value,
    #[serde(default)]
    pub questions: BTreeMap<String, DecisionQuestion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Parsed System 1 result. `checkpoint` is the routing model when the server reports one.
#[derive(Debug, Clone, PartialEq)]
pub struct System1Response {
    pub decider_id: String,
    pub checkpoint: Option<String>,
    pub answers: BTreeMap<String, DecisionAnswer>,
}

impl System1Response {
    /// Parse a Laya / Jev `/v1/systemone` JSON body against the questions that were asked.
    pub fn parse_wire(
        questions: &BTreeMap<String, DecisionQuestion>,
        value: &Value,
    ) -> Result<Self> {
        let answers_value = value.get("answers").unwrap_or(value);
        let answers_obj = answers_value
            .as_object()
            .ok_or_else(|| anyhow!("system 1 response is missing an answers object"))?;

        let mut answers = BTreeMap::new();
        for (name, question) in questions {
            let raw = answers_obj
                .get(name)
                .ok_or_else(|| anyhow!("system 1 response is missing an answer for `{name}`"))?;
            answers.insert(name.clone(), parse_answer(name, question, raw)?);
        }

        Ok(Self {
            decider_id: "wire".to_string(),
            checkpoint: checkpoint_from(value),
            answers,
        })
    }
}

/// Something that can run one System 1 forward pass.
///
/// Hosts attach a local checkpoint, an HTTP server (`laya-serve`, `sys1`, Jev),
/// or a test double. The memory reflex does not care which.
#[async_trait]
pub trait System1Decider: Send + Sync {
    fn decider_id(&self) -> &str;

    async fn predict(&self, request: &System1Request) -> Result<System1Response>;
}

fn checkpoint_from(value: &Value) -> Option<String> {
    if let Some(model) = value
        .get("routing")
        .and_then(|routing| routing.get("model"))
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
    {
        return Some(model.to_string());
    }
    value
        .get("model")
        .and_then(Value::as_str)
        .filter(|model| !model.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_answer(name: &str, question: &DecisionQuestion, value: &Value) -> Result<DecisionAnswer> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("system 1 answer `{name}` must be an object"))?;
    match question {
        DecisionQuestion::Choice { criteria, .. } => {
            let raw = obj
                .get("choice")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("system 1 answer `{name}` is missing `choice`"))?;
            let choice = canonicalize_choice(name, raw, criteria)?;
            let probabilities = probabilities_from(name, obj)?;
            let confidence = match obj.get("confidence") {
                Some(raw) => unit_interval(name, "confidence", as_f32(name, "confidence", raw)?)?,
                None => probabilities.get(&choice).copied().unwrap_or(0.0),
            };
            Ok(DecisionAnswer::Choice {
                choice,
                confidence,
                probabilities,
            })
        }
        DecisionQuestion::Score { .. } => {
            let score = as_f32(
                name,
                "score",
                obj.get("score")
                    .ok_or_else(|| anyhow!("system 1 answer `{name}` is missing `score`"))?,
            )?;
            if !score.is_finite() || score < 0.0 {
                bail!(
                    "system 1 answer `{name}` has a score that is not a non-negative finite number"
                );
            }
            let max = ["max", "out_of", "scale"]
                .iter()
                .find_map(|key| obj.get(*key))
                .map(|raw| as_f32(name, "max", raw))
                .transpose()?;
            if let Some(max) = max {
                if !max.is_finite() || max <= 0.0 {
                    bail!(
                        "system 1 answer `{name}` has a max that is not a positive finite number"
                    );
                }
            }
            let label = obj
                .get("label")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .map(ToOwned::to_owned);
            let confidence = match obj.get("confidence") {
                Some(raw) => unit_interval(name, "confidence", as_f32(name, "confidence", raw)?)?,
                None => 0.0,
            };
            Ok(DecisionAnswer::Score {
                score,
                max,
                label,
                confidence,
            })
        }
        DecisionQuestion::Noul { .. } => {
            let raw = obj
                .get("noul")
                .or_else(|| obj.get("probability"))
                .ok_or_else(|| anyhow!("system 1 answer `{name}` is missing `noul`"))?;
            let probability = unit_interval(name, "noul", as_f32(name, "noul", raw)?)?;
            Ok(DecisionAnswer::Noul { probability })
        }
    }
}

fn canonicalize_choice(
    name: &str,
    raw: &str,
    criteria: &BTreeMap<String, String>,
) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("system 1 answer `{name}` has an empty choice");
    }
    if criteria.is_empty() || criteria.contains_key(trimmed) {
        return Ok(trimmed.to_string());
    }
    if let Some(key) = criteria
        .keys()
        .find(|key| key.eq_ignore_ascii_case(trimmed))
    {
        return Ok(key.clone());
    }
    bail!(
        "system 1 answer `{name}` returned choice `{trimmed}`, which is outside the question criteria"
    );
}

fn probabilities_from(name: &str, obj: &Map<String, Value>) -> Result<BTreeMap<String, f32>> {
    let Some(raw) = obj.get("probabilities").or_else(|| obj.get("probs")) else {
        return Ok(BTreeMap::new());
    };
    let map = raw.as_object().ok_or_else(|| {
        anyhow!("system 1 answer `{name}` has probabilities that are not an object")
    })?;
    let mut out = BTreeMap::new();
    for (label, value) in map {
        let probability =
            unit_interval(name, "probabilities", as_f32(name, "probabilities", value)?)?;
        out.insert(label.clone(), probability);
    }
    Ok(out)
}

fn as_f32(name: &str, field: &str, value: &Value) -> Result<f32> {
    value
        .as_f64()
        .map(|number| number as f32)
        .ok_or_else(|| anyhow!("system 1 answer `{name}` field `{field}` must be a number"))
}

fn unit_interval(name: &str, field: &str, value: f32) -> Result<f32> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        bail!("system 1 answer `{name}` field `{field}` must be in 0..=1");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::{DecisionAnswer, DecisionQuestion, System1Response};

    fn sample_questions() -> BTreeMap<String, DecisionQuestion> {
        let mut questions = BTreeMap::new();
        questions.insert(
            "action".to_string(),
            DecisionQuestion::Choice {
                instructions: "which action?".to_string(),
                criteria: BTreeMap::from([
                    ("recall".to_string(), "read memory".to_string()),
                    ("ignore".to_string(), "do nothing".to_string()),
                ]),
            },
        );
        questions.insert(
            "salience".to_string(),
            DecisionQuestion::Score {
                instructions: "how salient?".to_string(),
                criteria: vec!["none".to_string(), "relevant".to_string()],
            },
        );
        questions.insert(
            "needs_system2".to_string(),
            DecisionQuestion::Noul {
                instructions: "escalate?".to_string(),
            },
        );
        questions
    }

    #[test]
    fn choice_question_matches_laya_wire_shape() {
        let questions = sample_questions();
        let value = serde_json::to_value(&questions["action"]).expect("question serializes");
        assert_eq!(value["type"], "choice");
        assert_eq!(value["criteria"]["recall"], "read memory");

        let salience = serde_json::to_value(&questions["salience"]).expect("score serializes");
        assert_eq!(salience["type"], "score");
        assert_eq!(salience["criteria"][0], "none");
    }

    #[test]
    fn parse_wire_reads_laya_answers_and_checkpoint() {
        let questions = sample_questions();
        let body = json!({
            "routing": {"model": "typed-decisions", "reason": "pinned"},
            "usage": {"input_tokens": 12, "output_tokens": 5},
            "answers": {
                "action": {
                    "choice": "Recall",
                    "confidence": 0.91,
                    "probabilities": {"recall": 0.91, "ignore": 0.09}
                },
                "salience": {"score": 2.1, "confidence": 0.8},
                "needs_system2": {"noul": 0.15}
            }
        });

        let parsed = System1Response::parse_wire(&questions, &body).expect("wire parses");
        assert_eq!(parsed.checkpoint.as_deref(), Some("typed-decisions"));
        match &parsed.answers["action"] {
            DecisionAnswer::Choice {
                choice, confidence, ..
            } => {
                assert_eq!(choice, "recall");
                assert!((*confidence - 0.91).abs() < 0.001);
            }
            other => panic!("expected choice, got {other:?}"),
        }
    }

    #[test]
    fn parse_wire_rejects_probability_above_one() {
        let questions = sample_questions();
        let body = json!({
            "answers": {
                "action": {"choice": "ignore", "confidence": 0.9},
                "salience": {"score": 0.2, "confidence": 0.9},
                "needs_system2": {"noul": 1.4}
            }
        });
        let error = System1Response::parse_wire(&questions, &body).unwrap_err();
        assert!(error.to_string().contains("needs_system2"));
    }
}
