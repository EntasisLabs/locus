pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
    tonic::include_file_descriptor_set!("sttp_descriptor");
pub(crate) const TENANT_HEADER: &str = "x-tenant-id";
pub(crate) const TENANT_HEADERS: [&str; 3] = ["x-resonantia-tenant", "x-tenant-id", "x-tenant"];
pub(crate) const DEFAULT_TENANT: &str = "default";
pub(crate) const TENANT_SCOPE_PREFIX: &str = "tenant:";
pub(crate) const TENANT_SCOPE_SEPARATOR: &str = "::session:";
pub(crate) const TENANT_SCAN_LIMIT: usize = 200;
pub(crate) const DEFAULT_HYBRID_ALPHA: f32 = 0.65;
pub(crate) const DEFAULT_HYBRID_BETA: f32 = 0.35;
