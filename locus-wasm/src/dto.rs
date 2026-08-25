use locus_core_rs::domain::models::{
    CanonicalAst, CanonicalAstLayer, ParseDiagnostic, ParseDiagnosticSeverity, ParseProfile,
    ParseResult, ParseSpan, StoreResult, ValidationFailureReason, ValidationResult,
};
use locus_sdk::interface::dto::{
    MemoryFindResponseDto, MemoryRecallResponseDto, MemorySchemaResponseDto,
};
use locus_sdk::{
    domain::compression::ManualCompressionResult, domain::memory::MemorySchemaResult,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDto {
    pub core: &'static str,
    pub sdk: &'static str,
    pub wasm: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseSpanDto {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseDiagnosticDto {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub strict_impact: bool,
    pub span: Option<ParseSpanDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalAstLayerDto {
    pub source: String,
    pub span: ParseSpanDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalAstDto {
    pub provenance: Option<CanonicalAstLayerDto>,
    pub envelope: Option<CanonicalAstLayerDto>,
    pub content: Option<CanonicalAstLayerDto>,
    pub metrics: Option<CanonicalAstLayerDto>,
    pub strict_spine: bool,
    pub profile: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResponseDto {
    pub success: bool,
    pub error: Option<String>,
    pub profile: String,
    pub strict_valid: bool,
    pub diagnostics: Vec<ParseDiagnosticDto>,
    pub canonical_ast: Option<CanonicalAstDto>,
    pub node: Option<locus_sdk::interface::dto::MemoryNodeDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResponseDto {
    pub is_valid: bool,
    pub error: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreResponseDto {
    pub node_id: String,
    pub psi: f32,
    pub valid: bool,
    pub validation_error: Option<String>,
}

pub fn version_dto() -> VersionDto {
    VersionDto {
        core: "0.5.1",
        sdk: "0.3.1",
        wasm: env!("CARGO_PKG_VERSION"),
    }
}

pub fn parse_response(result: ParseResult) -> ParseResponseDto {
    ParseResponseDto {
        success: result.success,
        error: result.error,
        profile: profile_name(result.profile).to_string(),
        strict_valid: result.strict_valid,
        diagnostics: result
            .diagnostics
            .into_iter()
            .map(parse_diagnostic)
            .collect(),
        canonical_ast: result.canonical_ast.map(canonical_ast),
        node: result.node.map(Into::into),
    }
}

pub fn validate_response(result: ValidationResult) -> ValidateResponseDto {
    ValidateResponseDto {
        is_valid: result.is_valid,
        error: result.error,
        reason: validation_reason(result.reason),
    }
}

pub fn store_response(result: StoreResult) -> StoreResponseDto {
    StoreResponseDto {
        node_id: result.node_id,
        psi: result.psi,
        valid: result.valid,
        validation_error: result.validation_error,
    }
}

pub fn schema_response(result: MemorySchemaResult) -> MemorySchemaResponseDto {
    result.into()
}

pub fn find_response(result: locus_sdk::domain::memory::MemoryFindResult) -> MemoryFindResponseDto {
    result.into()
}

pub fn recall_response(
    result: locus_sdk::domain::memory::MemoryRecallResult,
) -> MemoryRecallResponseDto {
    result.into()
}

pub fn compression_response(result: ManualCompressionResult) -> ManualCompressionResult {
    result
}

fn parse_diagnostic(diagnostic: ParseDiagnostic) -> ParseDiagnosticDto {
    ParseDiagnosticDto {
        code: diagnostic.code,
        message: diagnostic.message,
        severity: diagnostic_severity(diagnostic.severity).to_string(),
        strict_impact: diagnostic.strict_impact,
        span: diagnostic.span.map(parse_span),
    }
}

fn parse_span(span: ParseSpan) -> ParseSpanDto {
    ParseSpanDto {
        start: span.start,
        end: span.end,
        line: span.line,
        column: span.column,
    }
}

fn canonical_ast(ast: CanonicalAst) -> CanonicalAstDto {
    CanonicalAstDto {
        provenance: ast.provenance.map(canonical_layer),
        envelope: ast.envelope.map(canonical_layer),
        content: ast.content.map(canonical_layer),
        metrics: ast.metrics.map(canonical_layer),
        strict_spine: ast.strict_spine,
        profile: profile_name(ast.profile).to_string(),
    }
}

fn canonical_layer(layer: CanonicalAstLayer) -> CanonicalAstLayerDto {
    CanonicalAstLayerDto {
        source: layer.source,
        span: parse_span(layer.span),
    }
}

fn profile_name(profile: ParseProfile) -> &'static str {
    match profile {
        ParseProfile::Strict => "strict",
        ParseProfile::StrictTypedIr => "strictTypedIr",
        ParseProfile::Tolerant => "tolerant",
    }
}

fn diagnostic_severity(severity: ParseDiagnosticSeverity) -> &'static str {
    match severity {
        ParseDiagnosticSeverity::Fatal => "fatal",
        ParseDiagnosticSeverity::Error => "error",
        ParseDiagnosticSeverity::Warning => "warning",
        ParseDiagnosticSeverity::Info => "info",
    }
}

fn validation_reason(reason: ValidationFailureReason) -> String {
    reason.to_string()
}
