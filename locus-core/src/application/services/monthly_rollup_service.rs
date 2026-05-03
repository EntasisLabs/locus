use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;

use crate::domain::contracts::{NodeStore, NodeValidator};
use crate::domain::models::{
    AvecState, ConfidenceBandSummary, MonthlyRollupRequest, MonthlyRollupResult, NodeQuery,
    NumericRange,
};
use crate::parsing::SttpNodeParser;

pub struct MonthlyRollupService {
    store: Arc<dyn NodeStore>,
    validator: Arc<dyn NodeValidator>,
    parser: SttpNodeParser,
}

impl MonthlyRollupService {
    pub fn new(store: Arc<dyn NodeStore>, validator: Arc<dyn NodeValidator>) -> Self {
        Self {
            store,
            validator,
            parser: SttpNodeParser::new(),
        }
    }

    pub async fn create_async(&self, request: MonthlyRollupRequest) -> MonthlyRollupResult {
        if request.end_utc < request.start_utc {
            return MonthlyRollupResult {
                error: Some(
                    "InvalidRange: end must be greater than or equal to start.".to_string(),
                ),
                ..MonthlyRollupResult::default()
            };
        }

        let nodes = match self
            .store
            .query_nodes_async(NodeQuery {
                session_id: request.source_session_id.clone(),
                from_utc: Some(request.start_utc),
                to_utc: Some(request.end_utc),
                limit: request.limit,
                tiers: None,
            })
            .await
        {
            Ok(nodes) => nodes,
            Err(err) => {
                return MonthlyRollupResult {
                    error: Some(format!("QueryFailure: {err}")),
                    ..MonthlyRollupResult::default()
                };
            }
        };

        if nodes.is_empty() {
            return MonthlyRollupResult {
                error: Some("NoSourceNodes: no nodes found in the requested range.".to_string()),
                ..MonthlyRollupResult::default()
            };
        }

        let mut ordered_nodes = nodes;
        ordered_nodes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let user_nodes = ordered_nodes
            .iter()
            .filter(|n| n.user_avec.psi() > 0.0)
            .collect::<Vec<_>>();
        let model_nodes = ordered_nodes
            .iter()
            .filter(|n| n.model_avec.psi() > 0.0)
            .collect::<Vec<_>>();
        let compression_nodes = ordered_nodes
            .iter()
            .filter(|n| n.compression_avec.map(|avec| avec.psi()).unwrap_or(0.0) > 0.0)
            .collect::<Vec<_>>();

        let user_average = average_avec(user_nodes.iter().map(|n| n.user_avec));
        let model_average = average_avec(model_nodes.iter().map(|n| n.model_avec));
        let compression_average =
            average_avec(compression_nodes.iter().filter_map(|n| n.compression_avec));

        let rho_range = range_for(ordered_nodes.iter().map(|n| n.rho));
        let kappa_range = range_for(ordered_nodes.iter().map(|n| n.kappa));
        let psi_range = range_for(ordered_nodes.iter().map(|n| n.psi));
        let rho_bands = bands_for(ordered_nodes.iter().map(|n| n.rho));
        let kappa_bands = bands_for(ordered_nodes.iter().map(|n| n.kappa));
        let active_days = ordered_nodes
            .iter()
            .map(|n| n.timestamp.date_naive())
            .collect::<HashSet<_>>()
            .len();
        let parent_reference = request
            .parent_node_id
            .clone()
            .unwrap_or_else(|| ordered_nodes[0].session_id.clone());

        let raw_node = build_monthly_node(
            &request,
            &parent_reference,
            ordered_nodes.len(),
            user_nodes.len(),
            active_days,
            user_average,
            model_average,
            compression_average,
            rho_range,
            kappa_range,
            psi_range,
            rho_bands,
            kappa_bands,
        );

        let validation = self.validator.validate(&raw_node);
        if !validation.is_valid {
            return MonthlyRollupResult {
                raw_node,
                source_nodes: ordered_nodes.len(),
                parent_reference: Some(parent_reference),
                error: Some(format!(
                    "ValidationFailure: {}: {}",
                    validation.reason,
                    validation.error.unwrap_or_default()
                )),
                ..MonthlyRollupResult::default()
            };
        }

        let parse_result = self.parser.try_parse(&raw_node, &request.session_id);
        if !parse_result.success {
            return MonthlyRollupResult {
                raw_node,
                source_nodes: ordered_nodes.len(),
                parent_reference: Some(parent_reference),
                error: Some(format!(
                    "ParseFailure: {}",
                    parse_result.error.unwrap_or_default()
                )),
                ..MonthlyRollupResult::default()
            };
        }

        let mut node_id = String::new();
        if request.persist {
            let Some(parsed_node) = parse_result.node else {
                return MonthlyRollupResult {
                    raw_node,
                    source_nodes: ordered_nodes.len(),
                    parent_reference: Some(parent_reference),
                    error: Some("ParseFailure: missing parsed node".to_string()),
                    ..MonthlyRollupResult::default()
                };
            };

            match self.store.store_async(parsed_node).await {
                Ok(id) => node_id = id,
                Err(err) => {
                    return MonthlyRollupResult {
                        raw_node,
                        source_nodes: ordered_nodes.len(),
                        parent_reference: Some(parent_reference),
                        user_average,
                        model_average,
                        compression_average,
                        rho_range,
                        kappa_range,
                        psi_range,
                        rho_bands,
                        kappa_bands,
                        error: Some(format!("StoreFailure: {err}")),
                        ..MonthlyRollupResult::default()
                    };
                }
            }
        }

        MonthlyRollupResult {
            success: true,
            node_id,
            raw_node,
            source_nodes: ordered_nodes.len(),
            parent_reference: Some(parent_reference),
            user_average,
            model_average,
            compression_average,
            rho_range,
            kappa_range,
            psi_range,
            rho_bands,
            kappa_bands,
            ..MonthlyRollupResult::default()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_monthly_node(
    request: &MonthlyRollupRequest,
    parent_reference: &str,
    source_nodes: usize,
    source_user_avec_nodes: usize,
    active_days: usize,
    user_average: AvecState,
    model_average: AvecState,
    compression_average: AvecState,
    rho_range: NumericRange,
    kappa_range: NumericRange,
    psi_range: NumericRange,
    rho_bands: ConfidenceBandSummary,
    kappa_bands: ConfidenceBandSummary,
) -> String {
    let timestamp = Utc::now().to_rfc3339();
    let start = request.start_utc.format("%Y-%m-%d").to_string();
    let end = request.end_utc.format("%Y-%m-%d").to_string();
    let source_session_token = request
        .source_session_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(slug)
        .unwrap_or_else(|| "all_sessions".to_string());

    let template = r#"⊕⟨ ⏣0{ trigger: manual, response_format: temporal_node, origin_session: "__SESSION_ID__", compression_depth: 2, parent_node: ref:__PARENT_REFERENCE__, prime: { attractor_config: { stability: __USER_STABILITY__, friction: __USER_FRICTION__, logic: __USER_LOGIC__, autonomy: __USER_AUTONOMY__ }, context_summary: monthly_rollup_across_stored_sttp_nodes_with_average_state_and_confidence_spread, relevant_tier: monthly, retrieval_budget: 16 } } ⟩
⦿⟨ ⏣0{ timestamp: "__TIMESTAMP__", tier: monthly, session_id: "__SESSION_ID__", schema_version: "sttp-1.0", user_avec: { stability: __USER_STABILITY__, friction: __USER_FRICTION__, logic: __USER_LOGIC__, autonomy: __USER_AUTONOMY__, psi: __USER_PSI__ }, model_avec: { stability: __MODEL_STABILITY__, friction: __MODEL_FRICTION__, logic: __MODEL_LOGIC__, autonomy: __MODEL_AUTONOMY__, psi: __MODEL_PSI__ } } ⟩
◈⟨ ⏣0{ source_nodes(.99): __SOURCE_NODES__, source_user_avec_nodes(.97): __SOURCE_USER_AVEC_NODES__, active_days(.95): __ACTIVE_DAYS__, date_span(.99): __START___to___END__, source_session_filter(.78): __SOURCE_SESSION_TOKEN__, parent_anchor(.99): __PARENT_ANCHOR__, activity_shape(.83): burst_work_pattern_with_gaps_between_deep_sessions, monthly_arc(.86): stabilization_then_design_then_implementation_then_synthesis, behavioral_signature(.84): high_stability_high_logic_high_autonomy_with_low_to_moderate_friction, user_avec_average(.99): { stability: __USER_STABILITY__, friction: __USER_FRICTION__, logic: __USER_LOGIC__, autonomy: __USER_AUTONOMY__, psi: __USER_PSI__ }, model_avec_average(.97): { stability: __MODEL_STABILITY__, friction: __MODEL_FRICTION__, logic: __MODEL_LOGIC__, autonomy: __MODEL_AUTONOMY__, psi: __MODEL_PSI__ }, compression_avec_average(.96): { stability: __COMP_STABILITY__, friction: __COMP_FRICTION__, logic: __COMP_LOGIC__, autonomy: __COMP_AUTONOMY__, psi: __COMP_PSI__ }, confidence_ranges(.94): { rho_avg: __RHO_AVG__, rho_min: __RHO_MIN__, rho_max: __RHO_MAX__, kappa_avg: __KAPPA_AVG__, kappa_min: __KAPPA_MIN__, kappa_max: __KAPPA_MAX__, psi_avg: __PSI_AVG__, psi_min: __PSI_MIN__, psi_max: __PSI_MAX__ }, confidence_bands(.71): { rho_low: __RHO_LOW__, rho_medium: __RHO_MEDIUM__, rho_high: __RHO_HIGH__, kappa_low: __KAPPA_LOW__, kappa_medium: __KAPPA_MEDIUM__, kappa_high: __KAPPA_HIGH__ }, uncertainty(.41): interpretive_fields_carry_lower_confidence_than_numeric_rollups } ⟩
⍉⟨ ⏣0{ rho: __RHO_AVG__, kappa: __KAPPA_AVG__, psi: __PSI_AVG__, compression_avec: { stability: __COMP_STABILITY__, friction: __COMP_FRICTION__, logic: __COMP_LOGIC__, autonomy: __COMP_AUTONOMY__, psi: __COMP_PSI__ } } ⟩"#;

    template
        .replace("__SESSION_ID__", &request.session_id)
        .replace("__PARENT_REFERENCE__", parent_reference)
        .replace("__TIMESTAMP__", &timestamp)
        .replace("__SOURCE_NODES__", &source_nodes.to_string())
        .replace(
            "__SOURCE_USER_AVEC_NODES__",
            &source_user_avec_nodes.to_string(),
        )
        .replace("__ACTIVE_DAYS__", &active_days.to_string())
        .replace("__START__", &start)
        .replace("__END__", &end)
        .replace("__SOURCE_SESSION_TOKEN__", &source_session_token)
        .replace("__PARENT_ANCHOR__", &slug(parent_reference))
        .replace("__USER_STABILITY__", &format_float(user_average.stability))
        .replace("__USER_FRICTION__", &format_float(user_average.friction))
        .replace("__USER_LOGIC__", &format_float(user_average.logic))
        .replace("__USER_AUTONOMY__", &format_float(user_average.autonomy))
        .replace("__USER_PSI__", &format_float(user_average.psi()))
        .replace(
            "__MODEL_STABILITY__",
            &format_float(model_average.stability),
        )
        .replace("__MODEL_FRICTION__", &format_float(model_average.friction))
        .replace("__MODEL_LOGIC__", &format_float(model_average.logic))
        .replace("__MODEL_AUTONOMY__", &format_float(model_average.autonomy))
        .replace("__MODEL_PSI__", &format_float(model_average.psi()))
        .replace(
            "__COMP_STABILITY__",
            &format_float(compression_average.stability),
        )
        .replace(
            "__COMP_FRICTION__",
            &format_float(compression_average.friction),
        )
        .replace("__COMP_LOGIC__", &format_float(compression_average.logic))
        .replace(
            "__COMP_AUTONOMY__",
            &format_float(compression_average.autonomy),
        )
        .replace("__COMP_PSI__", &format_float(compression_average.psi()))
        .replace("__RHO_AVG__", &format_float(rho_range.average))
        .replace("__RHO_MIN__", &format_float(rho_range.min))
        .replace("__RHO_MAX__", &format_float(rho_range.max))
        .replace("__KAPPA_AVG__", &format_float(kappa_range.average))
        .replace("__KAPPA_MIN__", &format_float(kappa_range.min))
        .replace("__KAPPA_MAX__", &format_float(kappa_range.max))
        .replace("__PSI_AVG__", &format_float(psi_range.average))
        .replace("__PSI_MIN__", &format_float(psi_range.min))
        .replace("__PSI_MAX__", &format_float(psi_range.max))
        .replace("__RHO_LOW__", &rho_bands.low.to_string())
        .replace("__RHO_MEDIUM__", &rho_bands.medium.to_string())
        .replace("__RHO_HIGH__", &rho_bands.high.to_string())
        .replace("__KAPPA_LOW__", &kappa_bands.low.to_string())
        .replace("__KAPPA_MEDIUM__", &kappa_bands.medium.to_string())
        .replace("__KAPPA_HIGH__", &kappa_bands.high.to_string())
}

fn average_avec<I>(states: I) -> AvecState
where
    I: IntoIterator<Item = AvecState>,
{
    let values = states.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return AvecState::zero();
    }

    let len = values.len() as f32;
    let stability = values.iter().map(|s| s.stability).sum::<f32>() / len;
    let friction = values.iter().map(|s| s.friction).sum::<f32>() / len;
    let logic = values.iter().map(|s| s.logic).sum::<f32>() / len;
    let autonomy = values.iter().map(|s| s.autonomy).sum::<f32>() / len;

    AvecState {
        stability,
        friction,
        logic,
        autonomy,
    }
}

fn range_for<I>(values: I) -> NumericRange
where
    I: IntoIterator<Item = f32>,
{
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return NumericRange::default();
    }

    let min = values
        .iter()
        .fold(f32::INFINITY, |acc, value| acc.min(*value));
    let max = values
        .iter()
        .fold(f32::NEG_INFINITY, |acc, value| acc.max(*value));
    let average = values.iter().sum::<f32>() / values.len() as f32;

    NumericRange { min, max, average }
}

fn bands_for<I>(values: I) -> ConfidenceBandSummary
where
    I: IntoIterator<Item = f32>,
{
    let values = values.into_iter().collect::<Vec<_>>();
    ConfidenceBandSummary {
        low: values.iter().filter(|v| **v < 0.5).count(),
        medium: values.iter().filter(|v| **v >= 0.5 && **v < 0.85).count(),
        high: values.iter().filter(|v| **v >= 0.85).count(),
    }
}

fn format_float(value: f32) -> String {
    let mut s = format!("{value:.10}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() { "0".to_string() } else { s }
}

fn slug(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                output.push(lower);
            }
        } else {
            output.push('_');
        }
    }

    output.trim_matches('_').to_string()
}
