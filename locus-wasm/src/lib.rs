use locus_core_rs::application::validation::TreeSitterValidator;
use locus_core_rs::domain::contracts::NodeValidator;
use locus_core_rs::domain::models::ParseProfile;
use locus_core_rs::parsing::SttpNodeParser;
use wasm_bindgen::prelude::*;

mod client;
mod dto;
mod json;

pub use client::WasmLocusClient;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn version() -> Result<JsValue, JsValue> {
    json::to_value(&dto::version_dto())
}

#[wasm_bindgen]
pub fn parse_sttp(raw: &str, session_id: &str, profile: Option<String>) -> Result<JsValue, JsValue> {
    let result = match resolve_profile(profile.as_deref()) {
        ParseProfile::Strict => {
            SttpNodeParser::with_profile(ParseProfile::Strict).try_parse_strict(raw, session_id)
        }
        ParseProfile::StrictTypedIr => SttpNodeParser::with_profile(ParseProfile::StrictTypedIr)
            .try_parse_strict_typed_ir(raw, session_id),
        ParseProfile::Tolerant => {
            SttpNodeParser::with_profile(ParseProfile::Tolerant).try_parse_tolerant(raw, session_id)
        }
    };
    json::to_value(&dto::parse_response(result))
}

#[wasm_bindgen]
pub fn validate_sttp(raw: &str) -> Result<JsValue, JsValue> {
    let validator = TreeSitterValidator::new();
    let result = validator.validate(raw);
    json::to_value(&dto::validate_response(result))
}

#[wasm_bindgen]
pub fn memory_schema() -> Result<JsValue, JsValue> {
    client::memory_schema_value()
}

#[wasm_bindgen]
pub fn compress_text(request: JsValue) -> Result<JsValue, JsValue> {
    client::compress_text_value(request)
}

fn resolve_profile(profile: Option<&str>) -> ParseProfile {
    match profile
        .unwrap_or("tolerant")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "strict" => ParseProfile::Strict,
        "strict_typed_ir" | "stricttypedir" | "strict-typed-ir" => ParseProfile::StrictTypedIr,
        _ => ParseProfile::Tolerant,
    }
}
