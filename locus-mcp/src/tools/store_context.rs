use serde_json::json;

use crate::{
    SttpMcpServer, StoreContextRequest, infer_store_error_code, schema_first_guidance_payload,
    strict_typed_ir_profile_name, to_json_string,
};

pub(crate) async fn execute(server: &SttpMcpServer, request: StoreContextRequest) -> String {
    let result = server
        .store_context
        .store_async(&request.node, &request.session_id)
        .await;

    if !result.valid {
        let message = result.validation_error.unwrap_or_else(|| {
            "store_context rejected the payload under strict typed IR ingest policy".to_string()
        });
        let code = infer_store_error_code(&message);

        return to_json_string(json!({
            "node_id": result.node_id,
            "psi": result.psi,
            "valid": result.valid,
            "validation_error": message,
            "profile_policy": strict_typed_ir_profile_name(),
            "error": {
                "code": code,
                "message": message,
                "model_guidance": schema_first_guidance_payload(
                    "Inspect schema and strict ingest policy before retrying store_context."
                )
            }
        }));
    }

    to_json_string(json!({
        "node_id": result.node_id,
        "psi": result.psi,
        "valid": result.valid,
        "validation_error": result.validation_error,
        "profile_policy": strict_typed_ir_profile_name(),
    }))
}
