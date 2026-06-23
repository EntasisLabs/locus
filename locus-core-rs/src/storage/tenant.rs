const DEFAULT_TENANT: &str = "default";
const TENANT_SCOPE_PREFIX: &str = "tenant:";
const TENANT_SCOPE_SEPARATOR: &str = "::session:";

pub fn derive_tenant_id_from_session(session_id: &str) -> String {
    if let Some((tenant, _)) = session_id.split_once(TENANT_SCOPE_SEPARATOR) {
        if let Some(stripped) = tenant.strip_prefix(TENANT_SCOPE_PREFIX) {
            if !stripped.is_empty() {
                return stripped.to_string();
            }
        }
    }

    DEFAULT_TENANT.to_string()
}
