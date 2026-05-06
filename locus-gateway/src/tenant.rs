use axum::http::HeaderMap;

use locus_core_rs::domain::models as core_models;

use crate::constants::{
    DEFAULT_TENANT, TENANT_HEADER, TENANT_HEADERS, TENANT_SCOPE_PREFIX, TENANT_SCOPE_SEPARATOR,
};

pub(crate) fn normalize_tenant_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.to_ascii_lowercase();
    if normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Some(normalized)
    } else {
        None
    }
}

pub(crate) fn resolve_http_tenant(explicit_tenant: Option<&str>, headers: &HeaderMap) -> String {
    explicit_tenant
        .and_then(normalize_tenant_value)
        .or_else(|| resolve_tenant_header(headers))
        .unwrap_or_else(|| DEFAULT_TENANT.to_string())
}

pub(crate) fn resolve_grpc_tenant(metadata: &tonic::metadata::MetadataMap) -> String {
    metadata
        .get(TENANT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(normalize_tenant_value)
        .unwrap_or_else(|| DEFAULT_TENANT.to_string())
}

pub(crate) fn resolve_tenant_header(headers: &HeaderMap) -> Option<String> {
    TENANT_HEADERS.iter().find_map(|name| {
        headers
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .and_then(normalize_tenant_value)
    })
}

pub(crate) fn parse_scoped_session_id(session_id: &str) -> Option<(&str, &str)> {
    let remainder = session_id.strip_prefix(TENANT_SCOPE_PREFIX)?;
    remainder.split_once(TENANT_SCOPE_SEPARATOR)
}

pub(crate) fn is_default_tenant(tenant: &str) -> bool {
    tenant == DEFAULT_TENANT
}

pub(crate) fn scope_session_id(tenant: &str, session_id: &str) -> String {
    if is_default_tenant(tenant) {
        session_id.to_string()
    } else {
        format!("{TENANT_SCOPE_PREFIX}{tenant}{TENANT_SCOPE_SEPARATOR}{session_id}")
    }
}

pub(crate) fn session_belongs_to_tenant(session_id: &str, tenant: &str) -> bool {
    match parse_scoped_session_id(session_id) {
        Some((scoped_tenant, _)) => scoped_tenant == tenant,
        None => is_default_tenant(tenant),
    }
}

pub(crate) fn display_session_id(session_id: &str) -> String {
    match parse_scoped_session_id(session_id) {
        Some((_, base_session_id)) => base_session_id.to_string(),
        None => session_id.to_string(),
    }
}

pub(crate) fn normalize_node_for_tenant(
    mut node: core_models::SttpNode,
    tenant: &str,
) -> Option<core_models::SttpNode> {
    if !session_belongs_to_tenant(&node.session_id, tenant) {
        return None;
    }

    node.session_id = display_session_id(&node.session_id);
    Some(node)
}
