# Environment Setup

## Audience and Use

This page is for developers and platform teams preparing a reliable local or shared environment for Locus evaluation and integration.

Use it to:

1. Confirm supported tooling and runtime combinations.
2. Validate environment readiness before implementation work starts.
3. Standardize local, CI, and staging setup patterns.

## Support Matrix

| Component | Supported Baseline | Notes |
| --- | --- | --- |
| Rust toolchain | Stable channel | Use a pinned stable toolchain in CI for reproducibility. |
| Cargo | Bundled with Rust stable | Required for build, test, examples, and docs generation. |
| OS (development) | Linux, macOS, Windows (WSL2 recommended) | Linux and macOS are primary paths for host scripting. |
| Container runtime | Docker Engine or compatible runtime | Required for image-based MCP and gateway startup paths. |
| Storage backend | In-memory or SurrealDB v3 | In-memory for fast local validation; SurrealDB for persistent environments. |
| Docs toolchain | mdbook, mdbook-mermaid | Required for generated technical docs site. |

## Tooling Prerequisites

Install and verify:

```bash
rustc --version
cargo --version
docker --version
mdbook --version
mdbook-mermaid --version
```

If docs tools are missing:

```bash
cargo install mdbook
cargo install mdbook-mermaid
```

## Recommended Environment Profiles

### Local Development Profile

Use when iterating on features and integration code.

1. Storage: in-memory or local SurrealDB.
2. Host runtime: single process.
3. Objective: speed and repeatability.

### CI Validation Profile

Use for pull requests and release gating.

1. Storage: deterministic in-memory fixtures.
2. Host runtime: test-only jobs.
3. Objective: compatibility and regression detection.

### Staging Profile

Use for pre-production verification.

1. Storage: isolated remote SurrealDB.
2. Host runtime: gateway and MCP parity checks.
3. Objective: rollout rehearsal and migration validation.

## Baseline Verification

From repository root:

```bash
cargo check --workspace --examples
cargo test --workspace
cargo check --examples -p locus-sdk
./docs/build-technical-docs.sh
```

A setup is considered ready when all commands complete successfully.

## Runtime Smoke Start

Image-based MCP:

```bash
docker run --rm -i -v "$PWD/locus-data:/data" ghcr.io/entasislabs/locus-mcp:0.1.0
```

Image-based gateway:

```bash
docker run --rm -p 8080:8080 -p 8081:8081 -v "$PWD/locus-data:/data" ghcr.io/entasislabs/locus-gateway:2.0.0
```

## Environment Hygiene Controls

1. Keep secrets outside repository files.
2. Keep development and production data paths separate.
3. Pin model and provider settings in shared environments.
4. Use explicit tenant and session scope in integration tests.

## Exit Criteria

Environment setup is complete when:

1. Workspace build and tests pass.
2. At least one host runtime starts successfully.
3. Technical docs site builds successfully.
4. Team members can reproduce setup using this page only.
