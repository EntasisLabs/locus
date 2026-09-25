//! Reactive memory primitive.
//!
//! Passive primitives run when the caller already chose an operation. This
//! service is the step before that. A stimulus comes in — the payload an event
//! bus would have delivered — and an attached System 1 decider answers five
//! typed questions in one pass. The gate turns those answers into a
//! [`MemoryReflex`] envelope.
//!
//! The envelope is what a host publishes. This service does not subscribe,
//! publish, buffer, or touch a node store. Runnable recall, find, aggregate,
//! and persist payloads are filled only when the gate accepts the decision.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Result, bail};
use serde_json::json;

use crate::domain::memory::{
    MEMORY_SCHEMA_VERSION, MemoryAggregateRequest, MemoryFilter, MemoryFindRequest, MemoryGroupBy,
    MemoryPage, MemoryRecallRequest, clamp_limit,
};
use crate::domain::reflex::{
    MEMORY_ESCALATE_TOPIC, MemoryAction, MemoryPersistHint, MemoryPropositions, MemoryReflex,
    MemoryReflexKind, MemoryStimulus, ReflexGate, ReflexPolicy, SALIENCE_RUBRIC,
};
use crate::domain::system1::{
    DecisionAnswer, DecisionQuestion, System1Decider, System1Request, System1Response,
};
use crate::infrastructure::system1::HeuristicSystem1;

const QUESTION_ACTION: &str = "action";
const QUESTION_SALIENCE: &str = "salience";
const QUESTION_REFERENCES: &str = "references_prior";
const QUESTION_PERSIST: &str = "should_persist";
const QUESTION_SYSTEM2: &str = "needs_system2";

/// The five questions every memory reflex asks a System 1 model.
pub fn memory_reflex_questions() -> BTreeMap<String, DecisionQuestion> {
    let mut action = BTreeMap::new();
    action.insert(
        "ignore".to_string(),
        "acknowledgement, small talk, or a self-contained turn that should not read or write memory"
            .to_string(),
    );
    action.insert(
        "recall".to_string(),
        "the state depends on earlier conversation or asks to retrieve ranked prior context"
            .to_string(),
    );
    action.insert(
        "find".to_string(),
        "the state asks for a filtered lookup by phrase, tag, session, or time rather than ranked recall"
            .to_string(),
    );
    action.insert(
        "persist".to_string(),
        "the state states a durable fact, preference, correction, or instruction that should be stored"
            .to_string(),
    );
    action.insert(
        "explain".to_string(),
        "the state asks why a prior memory was used or how a remembered answer was grounded"
            .to_string(),
    );
    action.insert(
        "aggregate".to_string(),
        "the state asks for a count, summary, trend, or rollup across stored memory".to_string(),
    );

    BTreeMap::from([
        (
            QUESTION_ACTION.to_string(),
            DecisionQuestion::Choice {
                instructions: "Which single memory operation should run for this state? Pick ignore when the state has no durable or historical memory consequence.".to_string(),
                criteria: action,
            },
        ),
        (
            QUESTION_SALIENCE.to_string(),
            DecisionQuestion::Score {
                instructions: "How strongly must memory be touched before the next step can be correct?".to_string(),
                criteria: SALIENCE_RUBRIC.iter().map(|label| (*label).to_string()).collect(),
            },
        ),
        (
            QUESTION_REFERENCES.to_string(),
            DecisionQuestion::Noul {
                instructions: "Does this state depend on something already stored in memory?".to_string(),
            },
        ),
        (
            QUESTION_PERSIST.to_string(),
            DecisionQuestion::Noul {
                instructions: "Should a new durable memory be written from this state?".to_string(),
            },
        ),
        (
            QUESTION_SYSTEM2.to_string(),
            DecisionQuestion::Noul {
                instructions: "Is this too ambiguous, contradictory, or high-stakes for a typed memory decision alone?".to_string(),
            },
        ),
    ])
}

/// Gates System 1 answers into a memory envelope a host can put on its own bus.
pub struct MemoryReflexService {
    decider: Arc<dyn System1Decider>,
    policy: ReflexPolicy,
}

impl std::fmt::Debug for MemoryReflexService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemoryReflexService")
            .field("decider_id", &self.decider.decider_id())
            .field("policy", &self.policy)
            .finish()
    }
}

impl MemoryReflexService {
    pub fn new(decider: Arc<dyn System1Decider>) -> Self {
        Self {
            decider,
            policy: ReflexPolicy::default(),
        }
    }

    /// Attach the offline lexical decider. It speaks the catalog; it is not a checkpoint.
    pub fn heuristic() -> Self {
        Self::new(Arc::new(HeuristicSystem1))
    }

    pub fn with_policy(mut self, policy: ReflexPolicy) -> Self {
        self.policy = clamp_policy(policy);
        self
    }

    pub fn policy(&self) -> ReflexPolicy {
        self.policy
    }

    /// The exact System 1 request `decide` would send. A host that already ran
    /// Laya can POST this body, parse the response, and call [`Self::apply`].
    pub fn request_for(&self, stimulus: &MemoryStimulus) -> System1Request {
        System1Request {
            state: json!({
                "role": stimulus.role,
                "text": stimulus.text,
                "session_ids": stimulus.scope.session_ids,
                "metadata": stimulus.metadata,
            }),
            questions: memory_reflex_questions(),
            model: None,
        }
    }

    /// Run the attached decider, then gate the answers.
    pub async fn decide(&self, stimulus: &MemoryStimulus) -> Result<MemoryReflex> {
        if stimulus.text.trim().is_empty() {
            return Ok(blank_reflex(stimulus));
        }
        let decision = self.decider.predict(&self.request_for(stimulus)).await?;
        self.apply(stimulus, &decision)
    }

    /// Gate a forward pass the host already ran. The attached decider is not called.
    pub fn apply(
        &self,
        stimulus: &MemoryStimulus,
        decision: &System1Response,
    ) -> Result<MemoryReflex> {
        if stimulus.text.trim().is_empty() {
            return Ok(blank_reflex(stimulus));
        }
        let parsed = parse_decision(decision)?;
        let (kind, gate) = gate(&parsed, &self.policy);
        let companions = companions(kind, parsed.action, &parsed.propositions, &self.policy);
        let (recall, find, aggregate, persist) = if kind == MemoryReflexKind::Dispatch {
            payloads(stimulus, parsed.action, &companions, self.policy)
        } else {
            (None, None, None, None)
        };
        let topic = match kind {
            MemoryReflexKind::Escalate => MEMORY_ESCALATE_TOPIC.to_string(),
            MemoryReflexKind::Ignore => MemoryAction::Ignore.topic().to_string(),
            MemoryReflexKind::Dispatch => parsed.action.topic().to_string(),
        };
        let decider_id = if decision.decider_id.is_empty() {
            self.decider.decider_id().to_string()
        } else {
            decision.decider_id.clone()
        };

        Ok(MemoryReflex {
            schema_version: MEMORY_SCHEMA_VERSION.to_string(),
            stimulus_id: stimulus_id(stimulus),
            stimulus_text: stimulus.text.clone(),
            role: stimulus.role.clone(),
            scope: stimulus.scope.clone(),
            kind,
            action: parsed.action,
            topic,
            salience: parsed.salience,
            salience_label: parsed.salience_label,
            salience_confidence: parsed.salience_confidence,
            confidence: parsed.confidence,
            propositions: parsed.propositions,
            gate,
            companions,
            recall,
            find,
            aggregate,
            persist,
            decider_id,
            checkpoint: decision.checkpoint.clone(),
            metadata: stimulus.metadata.clone(),
        })
    }
}

struct ParsedDecision {
    action: MemoryAction,
    confidence: f32,
    salience: f32,
    salience_label: String,
    salience_confidence: f32,
    propositions: MemoryPropositions,
}

fn parse_decision(decision: &System1Response) -> Result<ParsedDecision> {
    let action_answer = required(decision, QUESTION_ACTION)?;
    let salience_answer = required(decision, QUESTION_SALIENCE)?;
    let references = noul(
        required(decision, QUESTION_REFERENCES)?,
        QUESTION_REFERENCES,
    )?;
    let should_persist = noul(required(decision, QUESTION_PERSIST)?, QUESTION_PERSIST)?;
    let needs_system2 = noul(required(decision, QUESTION_SYSTEM2)?, QUESTION_SYSTEM2)?;

    let (choice, confidence) = match action_answer {
        DecisionAnswer::Choice {
            choice, confidence, ..
        } => (choice, *confidence),
        _ => bail!("system 1 answer `{QUESTION_ACTION}` must be a choice"),
    };
    let action = MemoryAction::parse(choice)
        .ok_or_else(|| anyhow::anyhow!("system 1 choice `{choice}` is not a memory action"))?;

    let (score, max, label, salience_confidence) = match salience_answer {
        DecisionAnswer::Score {
            score,
            max,
            label,
            confidence,
        } => (*score, *max, label.clone(), *confidence),
        _ => bail!("system 1 answer `{QUESTION_SALIENCE}` must be a score"),
    };
    let rubric_max = (SALIENCE_RUBRIC.len() - 1) as f32;
    let max = max.filter(|value| *value > 0.0).unwrap_or(rubric_max);
    let salience = if max <= 0.0 {
        0.0
    } else {
        (score / max).clamp(0.0, 1.0)
    };
    let salience_label = label
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| rubric_label(score, max));

    Ok(ParsedDecision {
        action,
        confidence,
        salience,
        salience_label,
        salience_confidence,
        propositions: MemoryPropositions {
            references_prior: references,
            should_persist,
            needs_system2,
        },
    })
}

fn gate(parsed: &ParsedDecision, policy: &ReflexPolicy) -> (MemoryReflexKind, ReflexGate) {
    if parsed.propositions.needs_system2 >= policy.escalate_at {
        return (MemoryReflexKind::Escalate, ReflexGate::System2Required);
    }
    if parsed.confidence < policy.min_choice_confidence
        || parsed.salience_confidence < policy.min_choice_confidence
    {
        return (MemoryReflexKind::Escalate, ReflexGate::LowConfidence);
    }
    if parsed.salience < policy.min_salience {
        return if proposition_claims(parsed.action, &parsed.propositions, policy) {
            (
                MemoryReflexKind::Escalate,
                ReflexGate::PropositionDisagreement,
            )
        } else {
            (MemoryReflexKind::Ignore, ReflexGate::BelowSalience)
        };
    }
    match parsed.action {
        MemoryAction::Ignore => {
            if parsed.propositions.references_prior >= policy.read_floor
                || parsed.propositions.should_persist >= policy.write_floor
            {
                (
                    MemoryReflexKind::Escalate,
                    ReflexGate::PropositionDisagreement,
                )
            } else {
                (MemoryReflexKind::Ignore, ReflexGate::Accepted)
            }
        }
        MemoryAction::Persist => {
            if parsed.propositions.should_persist < policy.write_floor {
                (
                    MemoryReflexKind::Escalate,
                    ReflexGate::PropositionDisagreement,
                )
            } else {
                (MemoryReflexKind::Dispatch, ReflexGate::Accepted)
            }
        }
        MemoryAction::Recall
        | MemoryAction::Find
        | MemoryAction::Explain
        | MemoryAction::Aggregate => {
            if parsed.propositions.references_prior < policy.read_floor {
                (
                    MemoryReflexKind::Escalate,
                    ReflexGate::PropositionDisagreement,
                )
            } else {
                (MemoryReflexKind::Dispatch, ReflexGate::Accepted)
            }
        }
    }
}

fn proposition_claims(
    action: MemoryAction,
    propositions: &MemoryPropositions,
    policy: &ReflexPolicy,
) -> bool {
    match action {
        MemoryAction::Ignore => {
            propositions.references_prior >= policy.read_floor
                || propositions.should_persist >= policy.write_floor
        }
        MemoryAction::Persist => propositions.should_persist >= policy.write_floor,
        MemoryAction::Recall
        | MemoryAction::Find
        | MemoryAction::Explain
        | MemoryAction::Aggregate => propositions.references_prior >= policy.read_floor,
    }
}

fn companions(
    kind: MemoryReflexKind,
    action: MemoryAction,
    propositions: &MemoryPropositions,
    policy: &ReflexPolicy,
) -> Vec<MemoryAction> {
    if kind != MemoryReflexKind::Dispatch {
        return Vec::new();
    }
    let mut companions = Vec::new();
    if action.is_read() && propositions.should_persist >= policy.write_floor {
        companions.push(MemoryAction::Persist);
    }
    if action == MemoryAction::Persist && propositions.references_prior >= policy.read_floor {
        companions.push(MemoryAction::Recall);
    }
    companions
}

fn payloads(
    stimulus: &MemoryStimulus,
    action: MemoryAction,
    companions: &[MemoryAction],
    policy: ReflexPolicy,
) -> (
    Option<MemoryRecallRequest>,
    Option<MemoryFindRequest>,
    Option<MemoryAggregateRequest>,
    Option<MemoryPersistHint>,
) {
    let wants = |candidate: MemoryAction| action == candidate || companions.contains(&candidate);
    let recall = if wants(MemoryAction::Recall) || action == MemoryAction::Explain {
        Some(MemoryRecallRequest {
            scope: stimulus.scope.clone(),
            page: MemoryPage {
                limit: policy.page_limit,
                cursor: None,
            },
            query_text: Some(stimulus.text.clone()),
            ..Default::default()
        })
    } else {
        None
    };
    let find = if wants(MemoryAction::Find) {
        Some(MemoryFindRequest {
            scope: stimulus.scope.clone(),
            filter: MemoryFilter {
                text_contains: Some(stimulus.text.clone()),
                ..Default::default()
            },
            page: MemoryPage {
                limit: policy.page_limit,
                cursor: None,
            },
            ..Default::default()
        })
    } else {
        None
    };
    let aggregate = if wants(MemoryAction::Aggregate) {
        Some(MemoryAggregateRequest {
            scope: stimulus.scope.clone(),
            group_by: MemoryGroupBy::DateDay,
            max_groups: 31,
            max_nodes: 1000,
            ..Default::default()
        })
    } else {
        None
    };
    let persist = if wants(MemoryAction::Persist) {
        Some(MemoryPersistHint {
            text: stimulus.text.clone(),
            role: stimulus.role.clone(),
        })
    } else {
        None
    };
    (recall, find, aggregate, persist)
}

fn blank_reflex(stimulus: &MemoryStimulus) -> MemoryReflex {
    MemoryReflex {
        schema_version: MEMORY_SCHEMA_VERSION.to_string(),
        stimulus_id: stimulus_id(stimulus),
        stimulus_text: stimulus.text.clone(),
        role: stimulus.role.clone(),
        scope: stimulus.scope.clone(),
        kind: MemoryReflexKind::Ignore,
        action: MemoryAction::Ignore,
        topic: MemoryAction::Ignore.topic().to_string(),
        salience: 0.0,
        salience_label: SALIENCE_RUBRIC[0].to_string(),
        salience_confidence: 1.0,
        confidence: 1.0,
        propositions: MemoryPropositions::default(),
        gate: ReflexGate::BlankStimulus,
        companions: Vec::new(),
        recall: None,
        find: None,
        aggregate: None,
        persist: None,
        decider_id: "none".to_string(),
        checkpoint: None,
        metadata: stimulus.metadata.clone(),
    }
}

fn stimulus_id(stimulus: &MemoryStimulus) -> String {
    if let Some(id) = stimulus
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        return id.to_string();
    }
    let mut hash: u64 = 0xcbf29ce484222325;
    let mix = |hash: &mut u64, bytes: &[u8]| {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x100000001b3);
        }
        *hash ^= 0xff;
    };
    mix(&mut hash, stimulus.role.as_deref().unwrap_or("").as_bytes());
    if let Some(sessions) = &stimulus.scope.session_ids {
        for session in sessions {
            mix(&mut hash, session.as_bytes());
        }
    }
    mix(&mut hash, stimulus.text.as_bytes());
    format!("stim-{hash:016x}")
}

fn required<'a>(decision: &'a System1Response, name: &str) -> Result<&'a DecisionAnswer> {
    decision
        .answers
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("system 1 response is missing `{name}`"))
}

fn noul(answer: &DecisionAnswer, name: &str) -> Result<f32> {
    match answer {
        DecisionAnswer::Noul { probability } => Ok(*probability),
        _ => bail!("system 1 answer `{name}` must be a noul"),
    }
}

fn rubric_label(score: f32, max: f32) -> String {
    let steps = (SALIENCE_RUBRIC.len() - 1) as f32;
    let idx = if max <= 0.0 {
        0
    } else {
        (score / max * steps).round() as usize
    };
    SALIENCE_RUBRIC[idx.min(SALIENCE_RUBRIC.len() - 1)].to_string()
}

fn clamp_policy(policy: ReflexPolicy) -> ReflexPolicy {
    ReflexPolicy {
        min_choice_confidence: unit(policy.min_choice_confidence),
        min_salience: unit(policy.min_salience),
        read_floor: unit(policy.read_floor),
        write_floor: unit(policy.write_floor),
        escalate_at: unit(policy.escalate_at),
        page_limit: clamp_limit(policy.page_limit),
    }
}

fn unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;

    use super::{MemoryReflexService, memory_reflex_questions};
    use crate::domain::memory::MemoryScope;
    use crate::domain::reflex::{
        MemoryAction, MemoryReflexKind, MemoryStimulus, ReflexGate, ReflexPolicy,
    };
    use crate::domain::system1::{DecisionAnswer, System1Decider, System1Request, System1Response};
    use crate::interface::dto::MemoryReflexResponseDto;

    fn stimulus(text: &str) -> MemoryStimulus {
        MemoryStimulus {
            text: text.to_string(),
            role: Some("user".to_string()),
            scope: MemoryScope {
                session_ids: Some(vec!["s-1".to_string()]),
                ..Default::default()
            },
            metadata: serde_json::Map::from_iter([("correlation".to_string(), json!("c-1"))]),
            ..Default::default()
        }
    }

    fn scripted(
        action: &str,
        confidence: f32,
        salience: f32,
        salience_confidence: f32,
        references_prior: f32,
        should_persist: f32,
        needs_system2: f32,
    ) -> System1Response {
        let mut answers = std::collections::BTreeMap::new();
        answers.insert(
            "action".to_string(),
            DecisionAnswer::Choice {
                choice: action.to_string(),
                confidence,
                probabilities: std::collections::BTreeMap::new(),
            },
        );
        answers.insert(
            "salience".to_string(),
            DecisionAnswer::Score {
                score: salience * 3.0,
                max: Some(3.0),
                label: None,
                confidence: salience_confidence,
            },
        );
        for (name, probability) in [
            ("references_prior", references_prior),
            ("should_persist", should_persist),
            ("needs_system2", needs_system2),
        ] {
            answers.insert(name.to_string(), DecisionAnswer::Noul { probability });
        }
        System1Response {
            decider_id: "scripted".to_string(),
            checkpoint: Some("scripted".to_string()),
            answers,
        }
    }

    struct Bomb;

    #[async_trait]
    impl System1Decider for Bomb {
        fn decider_id(&self) -> &str {
            "bomb"
        }

        async fn predict(&self, _request: &System1Request) -> anyhow::Result<System1Response> {
            anyhow::bail!("decider should not run");
        }
    }

    #[tokio::test]
    async fn blank_stimulus_skips_the_decider() {
        let service = MemoryReflexService::new(Arc::new(Bomb));
        let reflex = service
            .decide(&stimulus("  \n"))
            .await
            .expect("blank ignores");
        assert_eq!(reflex.kind, MemoryReflexKind::Ignore);
        assert_eq!(reflex.gate, ReflexGate::BlankStimulus);
        assert!(reflex.recall.is_none());
        assert_eq!(reflex.decider_id, "none");
    }

    #[tokio::test]
    async fn apply_does_not_call_the_decider() {
        let service = MemoryReflexService::new(Arc::new(Bomb));
        let reflex = service
            .apply(
                &stimulus("do you remember the refund"),
                &scripted("recall", 0.92, 0.8, 0.9, 0.88, 0.1, 0.1),
            )
            .expect("apply gates a finished forward pass");
        assert_eq!(reflex.kind, MemoryReflexKind::Dispatch);
        assert_eq!(reflex.action, MemoryAction::Recall);
        assert_eq!(reflex.topic, "locus.memory.recall");
        assert_eq!(
            reflex
                .recall
                .and_then(|recall| recall.query_text)
                .as_deref(),
            Some("do you remember the refund")
        );
    }

    #[tokio::test]
    async fn heuristic_routes_the_catalog() {
        let service = MemoryReflexService::heuristic();
        let recall = service
            .decide(&stimulus("do you remember what we discussed about refunds"))
            .await
            .expect("recall");
        assert_eq!(recall.kind, MemoryReflexKind::Dispatch);
        assert_eq!(recall.action, MemoryAction::Recall);
        assert!(recall.salience_label.contains("relevant"));
        assert_eq!(recall.metadata.get("correlation"), Some(&json!("c-1")));
        assert_eq!(recall.recall.unwrap().page.limit, 8);

        let persist = service
            .decide(&stimulus("please remember that I prefer aisle seats"))
            .await
            .expect("persist");
        assert_eq!(persist.kind, MemoryReflexKind::Dispatch);
        assert_eq!(persist.action, MemoryAction::Persist);
        assert_eq!(
            persist.persist.unwrap().text,
            "please remember that I prefer aisle seats"
        );
        assert!(persist.recall.is_none());

        let find = service
            .decide(&stimulus("find the nodes tagged billing"))
            .await
            .expect("find");
        assert_eq!(find.action, MemoryAction::Find);
        assert_eq!(find.topic, "locus.memory.find");
        assert!(find.find.unwrap().filter.text_contains.is_some());

        let aggregate = service
            .decide(&stimulus("summarize what we decided over the last week"))
            .await
            .expect("aggregate");
        assert_eq!(aggregate.action, MemoryAction::Aggregate);
        assert!(aggregate.aggregate.is_some());

        let explain = service
            .decide(&stimulus("why did you recall that memory"))
            .await
            .expect("explain");
        assert_eq!(explain.action, MemoryAction::Explain);
        assert!(explain.recall.is_some());

        let thanks = service.decide(&stimulus("thanks")).await.expect("thanks");
        assert_eq!(thanks.kind, MemoryReflexKind::Ignore);
        assert_eq!(thanks.gate, ReflexGate::BelowSalience);
        assert!(thanks.recall.is_none());
        assert_eq!(thanks.topic, "locus.memory.ignore");

        let uncertain = service
            .decide(&stimulus(
                "the quarterly plan needs another look before friday",
            ))
            .await
            .expect("uncertain");
        assert_eq!(uncertain.kind, MemoryReflexKind::Escalate);
        assert_eq!(uncertain.gate, ReflexGate::System2Required);
        assert!(uncertain.recall.is_none());
        assert_eq!(uncertain.topic, "locus.memory.escalate");
    }

    #[test]
    fn low_confidence_and_disagreement_do_not_dispatch() {
        let service = MemoryReflexService::heuristic();
        let low = service
            .apply(
                &stimulus("hold this"),
                &scripted("recall", 0.2, 0.9, 0.9, 0.9, 0.1, 0.1),
            )
            .expect("low confidence");
        assert_eq!(low.kind, MemoryReflexKind::Escalate);
        assert_eq!(low.gate, ReflexGate::LowConfidence);
        assert!(low.recall.is_none());

        let split = service
            .apply(
                &stimulus("hold this"),
                &scripted("recall", 0.92, 0.8, 0.9, 0.1, 0.05, 0.1),
            )
            .expect("disagreement");
        assert_eq!(split.kind, MemoryReflexKind::Escalate);
        assert_eq!(split.gate, ReflexGate::PropositionDisagreement);

        let quiet = service
            .apply(
                &stimulus("hold this"),
                &scripted("recall", 0.92, 0.1, 0.9, 0.9, 0.1, 0.1),
            )
            .expect("low salience still claims memory");
        assert_eq!(quiet.kind, MemoryReflexKind::Escalate);
        assert_eq!(quiet.gate, ReflexGate::PropositionDisagreement);

        let drop = service
            .apply(
                &stimulus("hold this"),
                &scripted("ignore", 0.92, 0.1, 0.9, 0.1, 0.1, 0.1),
            )
            .expect("drop");
        assert_eq!(drop.kind, MemoryReflexKind::Ignore);
        assert_eq!(drop.gate, ReflexGate::BelowSalience);
        assert_eq!(drop.topic, "locus.memory.ignore");
    }

    #[test]
    fn persist_with_a_prior_reference_carries_a_recall_companion() {
        let service = MemoryReflexService::heuristic();
        let reflex = service
            .apply(
                &stimulus("save the preference and the earlier note"),
                &scripted("persist", 0.9, 0.8, 0.9, 0.8, 0.9, 0.1),
            )
            .expect("companion");
        assert_eq!(reflex.kind, MemoryReflexKind::Dispatch);
        assert_eq!(reflex.companions, vec![MemoryAction::Recall]);
        assert!(reflex.persist.is_some());
        assert!(reflex.recall.is_some());
    }

    #[test]
    fn laya_wire_body_round_trips_through_the_gate() {
        let service = MemoryReflexService::heuristic();
        let incoming = stimulus("do you remember the duplicate charge");
        let request = service.request_for(&incoming);
        assert_eq!(
            request.questions.keys().cloned().collect::<Vec<_>>(),
            memory_reflex_questions().into_keys().collect::<Vec<_>>()
        );
        let body = json!({
            "routing": {"model": "typed-decisions"},
            "answers": {
                "action": {"choice": "recall", "confidence": 0.94},
                "salience": {"score": 2.1, "confidence": 0.9},
                "references_prior": {"noul": 0.91},
                "should_persist": {"noul": 0.08},
                "needs_system2": {"noul": 0.12}
            }
        });
        let parsed = System1Response::parse_wire(&request.questions, &body).expect("parse");
        let reflex = service.apply(&incoming, &parsed).expect("gate");
        assert_eq!(reflex.checkpoint.as_deref(), Some("typed-decisions"));
        assert_eq!(reflex.kind, MemoryReflexKind::Dispatch);

        let wire = serde_json::to_value(MemoryReflexResponseDto::from(reflex)).expect("dto");
        let back: MemoryReflexResponseDto =
            serde_json::from_value(wire.clone()).expect("round trip");
        assert_eq!(back.topic, "locus.memory.recall");
        assert_eq!(back.schema_version, "locus-sdk.memory.v4");
        assert_eq!(
            back.recall.unwrap().query_text.as_deref(),
            Some("do you remember the duplicate charge")
        );
        assert_eq!(wire["scope"]["sessionIds"][0], "s-1");
    }

    #[test]
    fn unknown_choice_is_rejected() {
        let service = MemoryReflexService::heuristic().with_policy(ReflexPolicy {
            page_limit: 0,
            ..ReflexPolicy::default()
        });
        assert_eq!(service.policy().page_limit, 1);
        let error = service
            .apply(
                &stimulus("hold this"),
                &scripted("teleport", 0.99, 0.9, 0.9, 0.9, 0.1, 0.1),
            )
            .unwrap_err();
        assert!(error.to_string().contains("teleport"));
    }

    #[test]
    fn explicit_stimulus_id_is_kept() {
        let mut incoming = stimulus("thanks");
        incoming.id = Some(" bus-9 ".to_string());
        assert_eq!(super::stimulus_id(&incoming), "bus-9");
    }
}
