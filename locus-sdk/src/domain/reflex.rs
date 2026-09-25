//! Reactive memory envelope.
//!
//! The reflex is the value a host puts on its own event bus. Producing one does
//! not subscribe, publish, buffer, or open a store.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::memory::{
    MemoryAggregateRequest, MemoryFindRequest, MemoryRecallRequest, MemoryScope,
};

/// Ordered salience rubric shared by the reflex questions and every decider.
pub const SALIENCE_RUBRIC: [&str; 4] = [
    "none: memory would not change the outcome",
    "background: memory is optional color",
    "relevant: memory should be read or written",
    "blocking: the next step is wrong if memory is skipped",
];

/// Topic for decisions the gate will not run until a larger model reviews them.
pub const MEMORY_ESCALATE_TOPIC: &str = "locus.memory.escalate";

/// Memory operation a System 1 `choice` answer is allowed to name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAction {
    Ignore,
    Recall,
    Find,
    Persist,
    Explain,
    Aggregate,
}

impl MemoryAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Recall => "recall",
            Self::Find => "find",
            Self::Persist => "persist",
            Self::Explain => "explain",
            Self::Aggregate => "aggregate",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ignore" => Some(Self::Ignore),
            "recall" => Some(Self::Recall),
            "find" => Some(Self::Find),
            "persist" => Some(Self::Persist),
            "explain" => Some(Self::Explain),
            "aggregate" => Some(Self::Aggregate),
            _ => None,
        }
    }

    /// Stable topic name a host maps onto its bus. This crate never subscribes to it.
    pub fn topic(self) -> &'static str {
        match self {
            Self::Ignore => "locus.memory.ignore",
            Self::Recall => "locus.memory.recall",
            Self::Find => "locus.memory.find",
            Self::Persist => "locus.memory.persist",
            Self::Explain => "locus.memory.explain",
            Self::Aggregate => "locus.memory.aggregate",
        }
    }

    pub fn is_read(self) -> bool {
        matches!(
            self,
            Self::Recall | Self::Find | Self::Explain | Self::Aggregate
        )
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Ignore,
            Self::Recall,
            Self::Find,
            Self::Persist,
            Self::Explain,
            Self::Aggregate,
        ]
    }
}

/// What the host should do with the envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryReflexKind {
    /// Run the attached memory payload.
    Dispatch,
    /// Leave the store alone.
    Ignore,
    /// Ask a System 2 model before any memory primitive runs.
    Escalate,
}

/// Why the gate accepted, dropped, or held the System 1 choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReflexGate {
    Accepted,
    BlankStimulus,
    BelowSalience,
    LowConfidence,
    PropositionDisagreement,
    System2Required,
}

/// Calibrated propositions from the `noul` questions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPropositions {
    pub references_prior: f32,
    pub should_persist: f32,
    pub needs_system2: f32,
}

impl Default for MemoryPropositions {
    fn default() -> Self {
        Self {
            references_prior: 0.0,
            should_persist: 0.0,
            needs_system2: 0.0,
        }
    }
}

/// Text the host's persist subscriber should ingest. The reflex does not write a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPersistHint {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Inbound state. This is the body an event bus would have delivered.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStimulus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub scope: MemoryScope,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

/// Thresholds that turn a System 1 answer into a dispatch, an ignore, or an escalation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflexPolicy {
    /// Choice confidence, and salience confidence, below this escalate.
    pub min_choice_confidence: f32,
    /// Normalized salience below this ignores the stimulus.
    pub min_salience: f32,
    /// `references_prior` required before a read action can dispatch.
    pub read_floor: f32,
    /// `should_persist` required before a persist action can dispatch.
    pub write_floor: f32,
    /// `needs_system2` at or above this escalates before any other rule.
    pub escalate_at: f32,
    /// Page size copied onto recall and find payloads.
    pub page_limit: usize,
}

impl Default for ReflexPolicy {
    fn default() -> Self {
        Self {
            min_choice_confidence: 0.55,
            min_salience: 0.34,
            read_floor: 0.45,
            write_floor: 0.45,
            escalate_at: 0.70,
            page_limit: 8,
        }
    }
}

/// Bus envelope for one stimulus.
///
/// Runnable payloads are present only when `kind` is [`MemoryReflexKind::Dispatch`].
/// `companions` names extra operations whose propositions also cleared the floor,
/// so one message can say "recall, and also persist" without a second publish.
#[derive(Debug, Clone)]
pub struct MemoryReflex {
    pub schema_version: String,
    pub stimulus_id: String,
    pub stimulus_text: String,
    pub role: Option<String>,
    pub scope: MemoryScope,
    pub kind: MemoryReflexKind,
    pub action: MemoryAction,
    pub topic: String,
    pub salience: f32,
    pub salience_label: String,
    pub salience_confidence: f32,
    pub confidence: f32,
    pub propositions: MemoryPropositions,
    pub gate: ReflexGate,
    pub companions: Vec<MemoryAction>,
    pub recall: Option<MemoryRecallRequest>,
    pub find: Option<MemoryFindRequest>,
    pub aggregate: Option<MemoryAggregateRequest>,
    pub persist: Option<MemoryPersistHint>,
    pub decider_id: String,
    pub checkpoint: Option<String>,
    pub metadata: Map<String, Value>,
}
