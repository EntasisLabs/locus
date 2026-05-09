use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::contracts::{EmbeddingProvider, NodeStore, NodeValidator};
use crate::domain::models::{ParseDiagnostic, StoreResult};
use crate::parsing::SttpNodeParser;

pub struct StoreContextService {
    store: Arc<dyn NodeStore>,
    validator: Arc<dyn NodeValidator>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    parser: SttpNodeParser,
    retry_policy: StoreRetryPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct StoreRetryPolicy {
    pub max_failures_before_cooldown: u32,
    pub cooldown: Duration,
}

impl Default for StoreRetryPolicy {
    fn default() -> Self {
        Self {
            max_failures_before_cooldown: 3,
            cooldown: Duration::seconds(120),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SessionFailureState {
    consecutive_failures: u32,
    cooldown_until: Option<DateTime<Utc>>,
}

static SESSION_FAILURES: Lazy<Mutex<HashMap<String, SessionFailureState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

impl StoreContextService {
    /// Create a store-context service with validation but no embedding enrichment.
    pub fn new(store: Arc<dyn NodeStore>, validator: Arc<dyn NodeValidator>) -> Self {
        Self::new_with_policy(store, validator, StoreRetryPolicy::default())
    }

    /// Create a store-context service with validation and an explicit retry policy.
    pub fn new_with_policy(
        store: Arc<dyn NodeStore>,
        validator: Arc<dyn NodeValidator>,
        retry_policy: StoreRetryPolicy,
    ) -> Self {
        Self {
            store,
            validator,
            embedding_provider: None,
            parser: SttpNodeParser::new(),
            retry_policy,
        }
    }

    /// Create a store-context service with optional embedding enrichment.
    pub fn with_embedding_provider(
        store: Arc<dyn NodeStore>,
        validator: Arc<dyn NodeValidator>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self::with_embedding_provider_and_policy(
            store,
            validator,
            embedding_provider,
            StoreRetryPolicy::default(),
        )
    }

    /// Create a store-context service with embedding enrichment and explicit retry policy.
    pub fn with_embedding_provider_and_policy(
        store: Arc<dyn NodeStore>,
        validator: Arc<dyn NodeValidator>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        retry_policy: StoreRetryPolicy,
    ) -> Self {
        Self {
            store,
            validator,
            embedding_provider: Some(embedding_provider),
            parser: SttpNodeParser::new(),
            retry_policy,
        }
    }

    /// Validate, parse, optionally enrich, and persist a raw STTP node.
    pub async fn store_async(&self, node: &str, session_id: &str) -> StoreResult {
        emit_ingest_trace(
            session_id,
            "ingest",
            "attempt",
            &format!(
                "profile=StrictTypedIr validator=TreeSitter retry_policy=max_failures:{} cooldown_seconds:{}",
                self.retry_policy.max_failures_before_cooldown,
                self.retry_policy.cooldown.num_seconds()
            ),
        );

        if let Some(cooldown_until) = self.cooldown_until(session_id) {
            let state = self.failure_state_snapshot(session_id);
            emit_ingest_trace(
                session_id,
                "ingest",
                "cooldown_active",
                &format!(
                    "cooldown_until={} consecutive_failures={} content_redacted=true",
                    cooldown_until.to_rfc3339(),
                    state.map(|s| s.consecutive_failures).unwrap_or(0)
                ),
            );
            return StoreResult {
                node_id: String::new(),
                psi: 0.0,
                valid: false,
                validation_error: Some(format!(
                    "RateLimited: session is in cooldown until {}",
                    cooldown_until.to_rfc3339()
                )),
            };
        }

        let validation = self.validator.validate(node);
        if !validation.is_valid {
            let cooldown_until = self.record_failure(session_id);
            let state = self.failure_state_snapshot(session_id);
            emit_ingest_trace(
                session_id,
                "validator",
                &validation.reason.to_string(),
                &format!(
                    "error={} consecutive_failures={} cooldown_until={} content_redacted=true",
                    validation.error.as_deref().unwrap_or_default(),
                    state.map(|s| s.consecutive_failures).unwrap_or(0),
                    cooldown_until
                        .map(|until| until.to_rfc3339())
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
            return StoreResult {
                node_id: String::new(),
                psi: 0.0,
                valid: false,
                validation_error: Some(self.with_optional_cooldown(
                    format!(
                        "{}: {}",
                        validation.reason,
                        validation.error.unwrap_or_default()
                    ),
                    cooldown_until,
                )),
            };
        }

        let parse_result = self.parser.try_parse_strict_typed_ir(node, session_id);
        if !parse_result.success {
            let cooldown_until = self.record_failure(session_id);
            let state = self.failure_state_snapshot(session_id);
            emit_ingest_trace(
                session_id,
                "parser",
                "strict_parse_failure",
                &format!(
                    "profile={:?} strict_valid={} diagnostics={} consecutive_failures={} cooldown_until={} content_redacted=true",
                    parse_result.profile,
                    parse_result.strict_valid,
                    format_parse_diagnostics(&parse_result.diagnostics),
                    state.map(|s| s.consecutive_failures).unwrap_or(0),
                    cooldown_until
                        .map(|until| until.to_rfc3339())
                        .unwrap_or_else(|| "none".to_string())
                ),
            );
            return StoreResult {
                node_id: String::new(),
                psi: 0.0,
                valid: false,
                validation_error: Some(self.with_optional_cooldown(
                    format!("ParseFailure: {}", parse_result.error.unwrap_or_default()),
                    cooldown_until,
                )),
            };
        }

        let mut parsed = match parse_result.node {
            Some(node) => node,
            None => {
                let cooldown_until = self.record_failure(session_id);
                let state = self.failure_state_snapshot(session_id);
                emit_ingest_trace(
                    session_id,
                    "parser",
                    "missing_parsed_node",
                    &format!(
                        "strict parse returned success without node consecutive_failures={} cooldown_until={} content_redacted=true",
                        state.map(|s| s.consecutive_failures).unwrap_or(0),
                        cooldown_until
                            .map(|until| until.to_rfc3339())
                            .unwrap_or_else(|| "none".to_string())
                    ),
                );
                return StoreResult {
                    node_id: String::new(),
                    psi: 0.0,
                    valid: false,
                    validation_error: Some(self.with_optional_cooldown(
                        "ParseFailure: missing parsed node".to_string(),
                        cooldown_until,
                    )),
                };
            }
        };

        if let Some(provider) = self.embedding_provider.as_ref() {
            if let Some(embedding_input) =
                build_embedding_input(parsed.context_summary.as_deref(), &parsed.session_id)
            {
                if let Ok(vector) = provider.embed_async(&embedding_input).await {
                    parsed.embedding_dimensions = Some(vector.len());
                    parsed.embedding_model = Some(provider.model_name().to_string());
                    parsed.embedding = Some(vector);
                    parsed.embedded_at = Some(Utc::now());
                }
            }
        }

        match self.store.store_async(parsed.clone()).await {
            Ok(node_id) => {
                self.reset_failures(session_id);
                emit_ingest_trace(
                    session_id,
                    "store",
                    "ok",
                    &format!(
                        "node_persisted=true node_id={} psi={} content_redacted=true",
                        node_id, parsed.psi
                    ),
                );
                StoreResult {
                    node_id,
                    psi: parsed.psi,
                    valid: true,
                    validation_error: None,
                }
            }
            Err(err) => {
                let cooldown_until = self.record_failure(session_id);
                let state = self.failure_state_snapshot(session_id);
                emit_ingest_trace(
                    session_id,
                    "store",
                    "store_failure",
                    &format!(
                        "error={} consecutive_failures={} cooldown_until={} content_redacted=true",
                        err,
                        state.map(|s| s.consecutive_failures).unwrap_or(0),
                        cooldown_until
                            .map(|until| until.to_rfc3339())
                            .unwrap_or_else(|| "none".to_string())
                    ),
                );
                StoreResult {
                    node_id: String::new(),
                    psi: 0.0,
                    valid: false,
                    validation_error: Some(self.with_optional_cooldown(
                        format!("StoreFailure: {err}"),
                        cooldown_until,
                    )),
                }
            }
        }
    }

    fn cooldown_until(&self, session_id: &str) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        let failures = SESSION_FAILURES.lock().unwrap_or_else(|p| p.into_inner());
        failures
            .get(session_id)
            .and_then(|state| state.cooldown_until)
            .filter(|until| *until > now)
    }

    fn failure_state_snapshot(&self, session_id: &str) -> Option<SessionFailureState> {
        let failures = SESSION_FAILURES.lock().unwrap_or_else(|p| p.into_inner());
        failures.get(session_id).copied()
    }

    fn record_failure(&self, session_id: &str) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        let mut failures = SESSION_FAILURES.lock().unwrap_or_else(|p| p.into_inner());
        let state = failures
            .entry(session_id.to_string())
            .or_insert(SessionFailureState {
                consecutive_failures: 0,
                cooldown_until: None,
            });

        if state
            .cooldown_until
            .map(|until| until <= now)
            .unwrap_or(false)
        {
            state.cooldown_until = None;
        }

        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.consecutive_failures > self.retry_policy.max_failures_before_cooldown {
            let until = now + self.retry_policy.cooldown;
            state.cooldown_until = Some(until);
            return Some(until);
        }

        None
    }

    fn reset_failures(&self, session_id: &str) {
        let mut failures = SESSION_FAILURES.lock().unwrap_or_else(|p| p.into_inner());
        failures.remove(session_id);
    }

    fn with_optional_cooldown(
        &self,
        base: String,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> String {
        match cooldown_until {
            Some(until) => format!(
                "{base} (cooldown_active_until={})",
                until.to_rfc3339()
            ),
            None => base,
        }
    }
}

fn emit_ingest_trace(session_id: &str, stage: &str, reason: &str, detail: &str) {
    eprintln!(
        "[sttp_ingest_trace] session_id={} stage={} reason={} detail={}",
        session_id,
        stage,
        reason,
        detail
    );
}

fn format_parse_diagnostics(diagnostics: &[ParseDiagnostic]) -> String {
    if diagnostics.is_empty() {
        return "count=0".to_string();
    }

    let codes = diagnostics
        .iter()
        .map(|diag| diag.code.as_str())
        .collect::<Vec<_>>()
        .join("|");

    let messages = diagnostics
        .iter()
        .take(6)
        .map(|diag| sanitize_message(&diag.message))
        .collect::<Vec<_>>()
        .join(" || ");

    format!(
        "count={} codes={} messages={}",
        diagnostics.len(),
        codes,
        messages
    )
}

fn sanitize_message(message: &str) -> String {
    message
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '_' | '-' | '.' | ':' | '/' | '(' | ')' | '\'' | '"'))
        .collect::<String>()
}

fn build_embedding_input(context_summary: Option<&str>, session_id: &str) -> Option<String> {
    let summary = context_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let session = session_id.trim();

    if summary.is_none() && session.is_empty() {
        return None;
    }

    Some(match summary {
        Some(summary) if !session.is_empty() => format!("{summary}\nsession_id:{session}"),
        Some(summary) => summary,
        None => format!("session_id:{session}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::validation::TreeSitterValidator;
    use crate::storage::InMemoryNodeStore;

    const VALID_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "store-service-test", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "store service test", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "store-service-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { note(.99): "example" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

    const INVALID_STRICT_NODE: &str = r#"
⊕⟨ { trigger: manual, response_format: temporal_node, compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "store service test", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "store-service-test", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { note(.99): "example" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
"#;

    fn clear_session_failures() {
        let mut guard = SESSION_FAILURES.lock().unwrap_or_else(|p| p.into_inner());
        guard.clear();
    }

    #[tokio::test]
    async fn should_reject_node_when_strict_required_fields_missing() {
        clear_session_failures();
        let store = Arc::new(InMemoryNodeStore::new());
        let validator = Arc::new(TreeSitterValidator::new());
        let service = StoreContextService::new(store, validator);

        let result = service
            .store_async(INVALID_STRICT_NODE, "strict-required-fields")
            .await;

        assert!(!result.valid);
        let error = result.validation_error.unwrap_or_default();
        assert!(error.contains("ParseFailure"));
        assert!(error.contains("strict profile violation"));
    }

    #[tokio::test]
    async fn should_apply_session_cooldown_after_threshold_failures() {
        clear_session_failures();
        let store = Arc::new(InMemoryNodeStore::new());
        let validator = Arc::new(TreeSitterValidator::new());
        let service = StoreContextService::new_with_policy(
            store,
            validator,
            StoreRetryPolicy {
                max_failures_before_cooldown: 3,
                cooldown: Duration::seconds(60),
            },
        );
        let session_id = "cooldown-session";

        for _ in 0..4 {
            let result = service.store_async(INVALID_STRICT_NODE, session_id).await;
            assert!(!result.valid);
        }

        let blocked = service.store_async(VALID_NODE, session_id).await;
        assert!(!blocked.valid);
        assert!(
            blocked
                .validation_error
                .unwrap_or_default()
                .contains("RateLimited")
        );
    }
}
