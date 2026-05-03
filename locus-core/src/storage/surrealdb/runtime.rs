use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct SurrealDbEndpointsSettings {
    pub embedded: Option<String>,
    pub remote: Option<String>,
}

impl Default for SurrealDbEndpointsSettings {
    fn default() -> Self {
        Self {
            embedded: Some("surrealkv://data/sttp-mcp".to_string()),
            remote: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SurrealDbSettings {
    pub endpoints: SurrealDbEndpointsSettings,
    pub namespace: String,
    pub database: String,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Default for SurrealDbSettings {
    fn default() -> Self {
        Self {
            endpoints: SurrealDbEndpointsSettings::default(),
            namespace: "keryx".to_string(),
            database: "sttp-mcp".to_string(),
            user: Some("root".to_string()),
            password: Some("root".to_string()),
        }
    }
}

impl SurrealDbSettings {
    pub fn endpoint(&self, use_remote: bool) -> Result<String> {
        if use_remote {
            if let Some(remote) = self
                .endpoints
                .remote
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                return Ok(remote.clone());
            }
        }

        if let Some(embedded) = self
            .endpoints
            .embedded
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(embedded.clone());
        }

        let mode = if use_remote { "remote" } else { "embedded" };
        Err(anyhow!(
            "No SurrealDB endpoint configured for mode {mode}. Set SurrealDb Endpoints."
        ))
    }
}

#[derive(Debug, Clone)]
pub struct SurrealDbRuntimeOptions {
    pub root_dir: String,
    pub use_remote: bool,
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
}

impl SurrealDbRuntimeOptions {
    pub fn from_args(
        args: &[String],
        settings: &SurrealDbSettings,
        root_directory_name: Option<&str>,
    ) -> Result<Self> {
        let use_remote = args.iter().any(|arg| arg.eq_ignore_ascii_case("--remote"));
        let root_name = root_directory_name
            .unwrap_or(".sttp-mcp")
            .trim()
            .to_string();

        let home = env::var("HOME").map_err(|_| anyhow!("HOME is not set"))?;
        let root_dir = PathBuf::from(home).join(if root_name.is_empty() {
            ".sttp-mcp"
        } else {
            root_name.as_str()
        });

        let endpoint = settings.endpoint(use_remote)?;
        let normalized_endpoint = normalize_embedded_endpoint(&endpoint, &root_dir, use_remote)?;

        Ok(Self {
            root_dir: root_dir.to_string_lossy().to_string(),
            use_remote,
            endpoint: normalized_endpoint,
            namespace: settings.namespace.clone(),
            database: settings.database.clone(),
        })
    }
}

fn normalize_embedded_endpoint(
    endpoint: &str,
    root_dir: &Path,
    use_remote: bool,
) -> Result<String> {
    if use_remote {
        return Ok(endpoint.to_string());
    }

    const SCHEME: &str = "surrealkv://";
    if !endpoint.starts_with(SCHEME) {
        return Ok(endpoint.to_string());
    }

    let endpoint_path = endpoint.trim_start_matches(SCHEME);
    let absolute_path = if Path::new(endpoint_path).is_absolute() {
        PathBuf::from(endpoint_path)
    } else {
        root_dir.join(endpoint_path)
    };

    if let Some(parent) = absolute_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(format!("{SCHEME}{}", absolute_path.to_string_lossy()))
}
