use serde_json::json;
use tracing::error;

use crate::{CalibrateSessionRequest, SttpMcpServer, avec_to_json, to_json_string, tool_error};

pub(crate) async fn execute(server: &SttpMcpServer, request: CalibrateSessionRequest) -> String {
    match server
        .calibration
        .calibrate_async(
            &request.session_id,
            request.stability,
            request.friction,
            request.logic,
            request.autonomy,
            &request.trigger,
        )
        .await
    {
        Ok(result) => to_json_string(json!({
            "previous_avec": avec_to_json(result.previous_avec),
            "delta": result.delta,
            "drift_classification": format!("{:?}", result.drift_classification),
            "trigger": result.trigger,
            "trigger_history": result.trigger_history,
            "is_first_calibration": result.is_first_calibration,
        })),
        Err(err) => {
            error!(error = %err, "calibrate_session failed");
            tool_error("CalibrationFailure", &err.to_string())
        }
    }
}
