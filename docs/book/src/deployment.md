# Deployment

## Audience and Use

This page is for teams deploying Locus in development, staging, and production environments.

Use it to:

1. Select an operating model by environment.
2. Apply release checks before rollout.
3. Keep runtime behavior consistent across deployment targets.

For prerequisite tooling and baseline readiness checks, see `Environment Setup`.

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

## Platform and Runtime Support

| Area | Baseline |
| --- | --- |
| Rust | Stable toolchain |
| Host OS | Linux, macOS, Windows (WSL2 recommended) |
| Container runtime | Docker Engine or compatible runtime |
| Storage | In-memory, SurrealDB v3 |
| Docs build | mdbook, mdbook-mermaid |

## Build Commands

From repository root:

```bash
cargo check --workspace --examples
cargo test -p locus-core-rs --lib
cargo test -p locus-sdk
```

## Packaging

Core package helper:

```bash
./locus-core-rs/build-package.sh
```

Publish preflight helper:

```bash
./locus-core-rs/publish-crates.sh
```

## Configuration Guidance

1. Keep environment-specific endpoint credentials out of repository files.
2. Set explicit session/tenant scoping in host integrations.
3. Prefer pinned model/provider settings for repeatable production behavior.
4. Separate development and production provider endpoints.
5. Use explicit tenant/session scoping in host runtime configs.

These controls reduce configuration drift and support predictable operations during upgrades.

## Host Deployment Profiles

1. MCP host profile:
MCP prioritizes tool contract stability and strict parser and validator checks in smoke tests.

2. Gateway host profile:
Gateway prioritizes stable HTTP and gRPC mapping to SDK primitives with health checks and retrieval policy observability.

## Release Readiness Checklist

1. Workspace compile and tests pass.
2. Example binaries compile and execute in CI smoke path.
3. Integration hosts confirm SDK contract compatibility.
4. Changelog and migration notes updated.
5. Versioning and compatibility policy reviewed for this release.
6. Security checklist validated for secrets and logging hygiene.

Releases that do not meet this checklist should not be promoted to production.

## Release Gates by Environment

### Local Development Gate

1. Workspace compile succeeds.
2. Targeted crate tests pass for changed modules.
3. At least one example path executes without contract errors.

### CI Validation Gate

1. Workspace test matrix passes.
2. Lint and docs build complete in CI.
3. No parser or validator regressions in changed paths.

### Staging Gate

1. Gateway and MCP parity checks pass against the same dataset.
2. Retrieval path distribution stays within expected baseline variance.
3. Rollback procedure is rehearsed and time-bounded.

### Production Promotion Gate

1. Staging sign-off is recorded.
2. Observability alerts and dashboards are active.
3. Release note includes compatibility and rollback instructions.
