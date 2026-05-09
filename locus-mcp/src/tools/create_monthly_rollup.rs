use locus_core_rs::MonthlyRollupRequest;
use serde_json::json;

use crate::{
    CreateMonthlyRollupRequest, SttpMcpServer, avec_to_json, parse_utc_required, to_json_string,
    tool_error,
};

pub(crate) async fn execute(
    server: &SttpMcpServer,
    request: CreateMonthlyRollupRequest,
) -> String {
    let start_utc = match parse_utc_required(&request.start_date_utc, "start_date_utc") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidDate", &message),
    };
    let end_utc = match parse_utc_required(&request.end_date_utc, "end_date_utc") {
        Ok(value) => value,
        Err(message) => return tool_error("InvalidDate", &message),
    };

    let mut rollup_request = MonthlyRollupRequest::new(request.session_id, start_utc, end_utc);
    rollup_request.source_session_id = request.source_session_id;
    rollup_request.parent_node_id = request.parent_node_id;
    rollup_request.persist = request.persist;

    let result = server.monthly_rollup.create_async(rollup_request).await;
    if !result.success {
        let message = result
            .error
            .as_deref()
            .unwrap_or("Monthly rollup creation failed.");
        let code = if message.starts_with("InvalidRange") {
            "InvalidRange"
        } else {
            "MonthlyRollupFailure"
        };

        return tool_error(code, message);
    }

    to_json_string(json!({
        "success": result.success,
        "node_id": result.node_id,
        "raw_node": result.raw_node,
        "error": result.error,
        "source_nodes": result.source_nodes,
        "parent_reference": result.parent_reference,
        "user_average": avec_to_json(result.user_average),
        "model_average": avec_to_json(result.model_average),
        "compression_average": avec_to_json(result.compression_average),
        "rho_range": {
            "min": result.rho_range.min,
            "max": result.rho_range.max,
            "average": result.rho_range.average,
        },
        "kappa_range": {
            "min": result.kappa_range.min,
            "max": result.kappa_range.max,
            "average": result.kappa_range.average,
        },
        "psi_range": {
            "min": result.psi_range.min,
            "max": result.psi_range.max,
            "average": result.psi_range.average,
        },
        "rho_bands": {
            "low": result.rho_bands.low,
            "medium": result.rho_bands.medium,
            "high": result.rho_bands.high,
        },
        "kappa_bands": {
            "low": result.kappa_bands.low,
            "medium": result.kappa_bands.medium,
            "high": result.kappa_bands.high,
        },
    }))
}
