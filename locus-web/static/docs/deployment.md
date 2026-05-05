# Deployment

## Supported Runtime Modes

1. Embedded/local development mode.
2. Remote SurrealDB mode.
3. Hybrid host mode where gateway and MCP share the same underlying Locus behavior.

## Environment Matrix

| Environment | Storage Mode | Host Profile | Primary Goal |
| --- | --- | --- | --- |
| Local Development | In-memory or embedded Surreal | single-process | fast iteration and debugging |
| CI Validation | In-memory plus deterministic fixtures | test-only jobs | regression detection and compatibility checks |
| Staging | Remote SurrealDB with isolated tenant space | gateway plus mcp parity checks | release verification and migration rehearsal |
| Production | Remote SurrealDB with restricted access | managed gateway and mcp deployments | reliability, auditability, and scale |

## Build Prerequisites

1. Rust toolchain (stable).
2. cargo available in PATH.
3. Optional: SurrealDB endpoint for integration environments.

## Build Commands

From repository root:

```bash
cargo check --workspace --examples
cargo test -p locus-core --lib
cargo test -p locus-sdk
```

## Packaging

Core package helper:

```bash
./locus-core/build-package.sh
```

Publish preflight helper:

```bash
./locus-core/publish-crates.sh
```

## Configuration Guidance

1. Keep environment-specific endpoint credentials out of repository files.
2. Set explicit session/tenant scoping in host integrations.
3. Prefer pinned model/provider settings for repeatable production behavior.
4. Separate development and production provider endpoints.
5. Use explicit tenant/session scoping in host runtime configs.

## Host Deployment Profiles

1. MCP host profile:
1. focus on tool contract stability
2. include strict parser/validator checks in smoke tests

2. Gateway host profile:
1. enforce stable HTTP and gRPC contract mapping to SDK primitives
2. expose health checks and retrieval policy observability

## Release Readiness Checklist

1. Workspace compile and tests pass.
2. Example binaries compile and execute in CI smoke path.
3. Integration hosts confirm SDK contract compatibility.
4. Changelog and migration notes updated.
5. Versioning and compatibility policy reviewed for this release.
6. Security checklist validated for secrets and logging hygiene.
