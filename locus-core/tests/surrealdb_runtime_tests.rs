use locus_core::storage::surrealdb::{
    SurrealDbEndpointsSettings, SurrealDbRuntimeOptions, SurrealDbSettings,
};

#[test]
fn runtime_uses_remote_endpoint_when_flag_is_present() {
    let settings = SurrealDbSettings {
        endpoints: SurrealDbEndpointsSettings {
            embedded: Some("surrealkv://data/sttp-mcp".to_string()),
            remote: Some("ws://127.0.0.1:8000/rpc".to_string()),
        },
        ..SurrealDbSettings::default()
    };

    let args = vec!["--remote".to_string()];
    let runtime = SurrealDbRuntimeOptions::from_args(&args, &settings, Some(".locus-core-test"))
        .expect("runtime options should resolve");

    assert!(runtime.use_remote);
    assert_eq!(runtime.endpoint, "ws://127.0.0.1:8000/rpc");
}

#[test]
fn runtime_normalizes_embedded_relative_path() {
    let settings = SurrealDbSettings {
        endpoints: SurrealDbEndpointsSettings {
            embedded: Some("surrealkv://data/sttp-mcp".to_string()),
            remote: None,
        },
        ..SurrealDbSettings::default()
    };

    let runtime =
        SurrealDbRuntimeOptions::from_args(&[], &settings, Some(".locus-core-test-embedded"))
            .expect("runtime options should resolve");

    assert!(!runtime.use_remote);
    assert!(runtime.endpoint.starts_with("surrealkv://"));
    assert!(runtime.endpoint.contains(".locus-core-test-embedded"));
}
