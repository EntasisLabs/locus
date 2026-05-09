use serde_json::json;

use crate::{GetMoodsRequest, SttpMcpServer, avec_to_json, to_json_string};

pub(crate) async fn execute(server: &SttpMcpServer, request: GetMoodsRequest) -> String {
    let result = server.moods.get(
        request.target_mood.as_deref(),
        request.blend,
        request.current_stability,
        request.current_friction,
        request.current_logic,
        request.current_autonomy,
    );

    let swap_preview = result.swap_preview.as_ref().map(|preview| {
        json!({
            "target_mood": preview.target_mood,
            "blend": preview.blend,
            "current": avec_to_json(preview.current),
            "target": avec_to_json(preview.target),
            "blended": avec_to_json(preview.blended),
        })
    });

    to_json_string(json!({
        "presets": result
            .presets
            .iter()
            .map(|preset| {
                json!({
                    "name": preset.name,
                    "description": preset.description,
                    "avec": avec_to_json(preset.avec),
                })
            })
            .collect::<Vec<_>>(),
        "apply_guide": result.apply_guide,
        "swap_preview": swap_preview,
    }))
}
