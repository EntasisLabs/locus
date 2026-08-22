/// Whether the endpoint targets a remote SurrealDB transport.
pub fn is_remote_endpoint(endpoint: &str) -> bool {
    let lower = endpoint.trim().to_ascii_lowercase();
    lower.starts_with("ws://")
        || lower.starts_with("wss://")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
}

/// Whether the endpoint targets an embedded SurrealDB engine.
pub fn is_embedded_endpoint(endpoint: &str) -> bool {
    let lower = endpoint.trim().to_ascii_lowercase();
    lower.starts_with("indxdb://")
        || lower.starts_with("mem://")
        || lower.starts_with("surrealkv://")
}

/// Resolve whether authentication and remote transport semantics apply.
pub fn effective_use_remote(endpoint: &str, use_remote: bool) -> bool {
    if is_embedded_endpoint(endpoint) {
        return false;
    }
    if is_remote_endpoint(endpoint) {
        return true;
    }
    use_remote
}

#[cfg(test)]
mod tests {
    use super::{effective_use_remote, is_embedded_endpoint, is_remote_endpoint};

    #[test]
    fn detects_indexeddb_endpoint() {
        assert!(is_embedded_endpoint("indxdb://locus"));
        assert!(!is_remote_endpoint("indxdb://locus"));
        assert!(!effective_use_remote("indxdb://locus", true));
    }

    #[test]
    fn detects_websocket_endpoint() {
        assert!(is_remote_endpoint("wss://example.com/rpc"));
        assert!(effective_use_remote("wss://example.com/rpc", false));
    }
}
