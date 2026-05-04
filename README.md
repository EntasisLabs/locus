# Locus

**Intelligence requires a place to stand.**

Language models are stateless by default, existing in a vacuum of the "now." To move beyond fleeting chat sessions and into enduring agency, information requires a stable environment. In the ancient **Method of Loci**, memory was mastered by anchoring ideas to physical landmarks turning abstract thoughts into a navigable palace.

**Locus** is the architectural realization of that palace for the STTP protocol.

It is the infrastructure of "where." By providing a standalone implementation layer for memory operations, Locus ensures that context is no longer a transient variable, but a persistent coordinate. Whether accessed via a gateway, a terminal, or an embedded surface, the memory remains spatially consistent and protocol-aligned.

We build Locus because for an agent to reason, it must first remember; and to remember, it must have a place to return to.

---

How this connects to STTP:

- **The Spatial Anchor:** Just as the Method of Loci uses _loci_ (places) to store data, Locus uses the STTP protocol to define the "geometry" of memory—making context retrievable through structural navigation rather than just keyword searching.
- **Spatio-Temporal Continuity:** By decoupling the memory layer, we ensure that the "Spatio" (the location of the data) and the "Temporal" (the persistence over time) are preserved regardless of which model or interface is interacting with it.
- **Infrastructure over Magic:** Locus does not offer "vague magic." It offers the raw, visible machinery of a memory palace, giving both developers and agents a living system to reason within.



Licensed under [Apache-2.0](LICENSE).

## Who This Is For

Locus is useful if you are one of the following:

1. Platform engineer needing deployable memory services over HTTP/gRPC or MCP.
2. Application engineer embedding memory primitives directly in Rust code.
3. AI tooling engineer integrating persistent context retrieval into model workflows.
4. Non-technical and semi-technical users exploring AI agents who want simple memory setup with minimal infrastructure overhead.
5. Release manager owning independent component version lines and artifacts.

Locus is not a product UI and not a single opinionated deployment framework. It is infrastructure you can compose.

For non-technical users, the intended entry path is:

1. Start with MCP using containerized defaults.
2. Use prebuilt images and standard tool flows first.
3. Move to CLI Skills workflows as that surface is added.

## Why This Moment Matters

AI usage is shifting from prompt-only chat sessions to always-on agent workflows.

In that shift, memory and reusable action surfaces become foundational:

1. Memory keeps continuity across sessions, tools, and models.
2. Skills convert one-off prompt behavior into repeatable operational actions.
3. Lightweight local-first setup lowers the barrier for non-technical adoption.

Locus is being shaped for that direction: strong memory first, with MCP today and CLI Skills support as the next usability layer.

## What Ships In This Repository

Workspace crates:

1. [locus-core/README.md](locus-core/README.md): parser/validator, domain contracts, storage abstractions, retrieval services, sync-ready mechanics.
2. [locus-sdk/README.md](locus-sdk/README.md): primitive-first SDK and composition workflows.
3. [locus-mcp/README.md](locus-mcp/README.md): stdio MCP server exposing memory tools.
4. [locus-gateway/README.md](locus-gateway/README.md): deployable Rust gateway with HTTP + gRPC.
5. [locus-cli/README.md](locus-cli/README.md): operator-friendly SDK-backed CLI for common memory workflows.

Operational docs:

1. [docs/README.md](docs/README.md)
2. [docs/architecture.md](docs/architecture.md)
3. [docs/deployment.md](docs/deployment.md)
4. [docs/operations.md](docs/operations.md)
5. [docs/integration.md](docs/integration.md)
6. [docs/examples.md](docs/examples.md)
7. [docs/troubleshooting.md](docs/troubleshooting.md)
8. [docs/versioning.md](docs/versioning.md)
9. [docs/security.md](docs/security.md)
10. [docs/sttp_typed_ir_language_spec.md](docs/sttp_typed_ir_language_spec.md)

## Start Path By Goal

If you are deciding where to begin:

1. I need a Rust API now: start with [locus-core/README.md](locus-core/README.md) and [locus-sdk/README.md](locus-sdk/README.md).
2. I need memory tools in an MCP client: start with [locus-mcp/README.md](locus-mcp/README.md).
3. I need a network host for apps/services: start with [locus-gateway/README.md](locus-gateway/README.md).
4. I want terminal-first workflows without writing service code: start with [locus-cli/README.md](locus-cli/README.md).
5. I am new to infra and want simple setup: start with the image-based quick runs in this README, then [locus-mcp/README.md](locus-mcp/README.md).
6. I need operational and release policy: start with [docs/deployment.md](docs/deployment.md), [docs/operations.md](docs/operations.md), and [docs/versioning.md](docs/versioning.md).

## Runtime Strategy

Recommended adoption order is:

1. Published images for immediate use.
2. Release binaries for platform-specific deployment.
3. Source builds for development and migration work.

This keeps first usage simple while preserving full control for teams that need custom builds.

## Skills And CLI Direction

Locus is expanding toward Skills-style CLI workflows for easier automation composition.

Planned intent:

1. Keep memory operations accessible for users who do not want to write service code.
2. Support reusable task patterns that can run repeatedly with stable memory context.
3. Preserve parity between MCP-based flows and future CLI Skills flows.

Current status:

1. MCP is production-ready as the easiest non-code entry point.
2. CLI Skills support is the next adoption layer and is being added with the same contract-first approach.

## Quick Start

From repository root:

```bash
cargo check --workspace
cargo test --workspace
cargo check --examples -p locus-sdk
```

## Fast Run Modes

### Run With Published Images

MCP server:

```bash
docker run --rm -i -v "$PWD/locus-data:/data" ghcr.io/keryxlabs/locus-mcp:0.1.0
```

Gateway:

```bash
docker run --rm -p 8080:8080 -p 8081:8081 -v "$PWD/locus-data:/data" ghcr.io/keryxlabs/locus-gateway:2.0.0
```

Notes:

1. Replace tags with the release you intend to run.
2. Use mounted storage for persistent host state.
3. Keep dev and production data paths separate.

### Run Release Binary Builds

MCP multi-target archives:

```bash
./locus-mcp/build.sh
```

Gateway multi-target archives:

```bash
./locus-gateway/build.sh
```

Both scripts support `--publish` to upload packaged artifacts to the matching namespaced GitHub release tag.

### Build From Source (Dev)

Run MCP locally:

```bash
LOCUS_MCP_IN_MEMORY=true cargo run --manifest-path locus-mcp/Cargo.toml
```

Run Gateway locally:

```bash
cargo run --manifest-path locus-gateway/Cargo.toml
```

Run SDK examples:

```bash
cargo run -p locus-sdk --example provider_registry_setup
cargo run -p locus-sdk --example memory_composition
cargo run -p locus-sdk --example recursive_composite_pipeline
```

Run CLI help:

```bash
cargo run -p locus-cli -- --help
```

## SDK, MCP, And Gateway At A Practical Level

## SDK Usage Summary

Use [locus-sdk/README.md](locus-sdk/README.md) when you want transport-agnostic memory behavior in-process.

Core primitives:

1. `memory_find`: deterministic filtering/sorting.
2. `memory_recall`: ranked retrieval using AVEC and optional semantic signals.
3. `memory_aggregate`: grouped stats and rollups.
4. `memory_transform`: controlled mutation/backfill workflows.
5. `memory_explain`: visibility into recall decisions.
6. `memory_schema`: runtime capability introspection.

Composition workflows:

1. `recall_with_explain`
2. `daily_rollup`
3. `transform_then_recall_verify`
4. `capability_bundle`
5. `build_content_from_text`

Typical SDK integration sequence:

1. Start with `memory_recall` and `memory_find` for base retrieval.
2. Add `memory_explain` when auditable reasoning is required.
3. Add composition workflows only for recurring multi-step operations.
4. Keep scoring and fallback policy explicit in request payloads.

### MCP Usage Summary

Use [locus-mcp/README.md](locus-mcp/README.md) when memory should be exposed through stdio tools to assistants/agents.

Primary tools:

1. `calibrate_session`
2. `store_context`
3. `get_context`
4. `list_nodes`
5. `get_moods`
6. `create_monthly_rollup`

Common MCP flow:

1. Calibrate current AVEC state.
2. Store checkpointed context.
3. Retrieve resonant context for the current state.
4. Inspect node inventory if needed.
5. Roll up historical windows when timeline density grows.

### Gateway Usage Summary

Use [locus-gateway/README.md](locus-gateway/README.md) when memory needs to be consumed by services over HTTP/gRPC.

Common HTTP sequence:

1. `GET /health`
2. `POST /api/v1/calibrate`
3. `POST /api/v1/store`
4. `POST /api/v1/context`
5. `GET /api/v1/nodes`

Gateway behavior emphasis:

1. Stable endpoint compatibility during internals migration.
2. Tenant-aware scoping with default backward-compatible behavior.
3. Sync-ready storage support without forcing sync policy decisions.

## End-To-End Interaction Examples

### Gateway Health + Calibrate + Store + Context

```bash
curl -s http://127.0.0.1:8080/health

curl -s -X POST http://127.0.0.1:8080/api/v1/calibrate \
	-H 'content-type: application/json' \
	-d '{
		"sessionId":"readme-demo",
		"stability":0.82,
		"friction":0.28,
		"logic":0.90,
		"autonomy":0.78,
		"trigger":"manual"
	}'

curl -s -X POST http://127.0.0.1:8080/api/v1/store \
	-H 'content-type: application/json' \
	-d '{
		"sessionId":"readme-demo",
		"node":"⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: \"readme-demo\", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.82, friction: 0.28, logic: 0.90, autonomy: 0.78 }, context_summary: \"readme demo node\", relevant_tier: raw, retrieval_budget: 5 } } ⟩\n⦿⟨ { timestamp: \"2026-05-03T00:00:00Z\", tier: raw, session_id: \"readme-demo\", user_avec: { stability: 0.82, friction: 0.28, logic: 0.90, autonomy: 0.78, psi: 2.78 }, model_avec: { stability: 0.82, friction: 0.28, logic: 0.90, autonomy: 0.78, psi: 2.78 } } ⟩\n◈⟨ { summary(.98): \"readme demo node\" } ⟩\n⍉⟨ { rho: 0.95, kappa: 0.93, psi: 2.78, compression_avec: { stability: 0.82, friction: 0.28, logic: 0.90, autonomy: 0.78, psi: 2.78 } } ⟩"
	}'

curl -s -X POST http://127.0.0.1:8080/api/v1/context \
	-H 'content-type: application/json' \
	-d '{
		"sessionId":"readme-demo",
		"stability":0.82,
		"friction":0.28,
		"logic":0.90,
		"autonomy":0.78,
		"limit":5
	}'
```

### SDK Example Execution

```bash
cargo run -p locus-sdk --example memory_composition
```

What this gives you:

1. Recall with explain output.
2. Daily rollup sample.
3. Capability bundle output.
4. Transform-then-recall verification path.

### MCP Example Operational Flow

For an MCP-capable assistant client:

1. Call `calibrate_session` with current AVEC state.
2. Call `store_context` with one valid STTP node.
3. Call `get_context` with the same session and AVEC state.
4. Call `list_nodes` when inventory review is needed.
5. Call `create_monthly_rollup` for timeline compaction.

## Build, Tags, And Release Process

Locus uses Instrumenta-style namespaced component release lines.

### Tag Prefixes

1. `locus-core/v...`
2. `locus-sdk/v...`
3. `locus-mcp/v...`
4. `locus-gateway/v...`

### Artifact Matrix

| Component | Artifact Type | Build Command | Publish Action |
| --- | --- | --- | --- |
| `locus-core` | crates.io package | `./locus-core/publish-crates.sh` | add `--publish` for actual crates.io publish |
| `locus-sdk` | crates.io package | `cargo publish --manifest-path locus-sdk/Cargo.toml --dry-run` | rerun without `--dry-run` |
| `locus-mcp` | multi-platform archives | `./locus-mcp/build.sh` | `./locus-mcp/build.sh --publish` |
| `locus-gateway` | multi-platform archives | `./locus-gateway/build.sh` | `./locus-gateway/build.sh --publish` |
| `locus-cli` | multi-platform archives | `./locus-cli/build.sh` | `./locus-cli/build.sh --publish` uploads assets to `locus-cli/vX.Y.Z` |
| `locus-mcp` | Docker image | `./locus-mcp/build-image.sh ghcr.io/keryxlabs/locus-mcp:X.Y.Z` | `docker push ghcr.io/keryxlabs/locus-mcp:X.Y.Z` |
| `locus-gateway` | Docker image | `./locus-gateway/build-image.sh ghcr.io/keryxlabs/locus-gateway:X.Y.Z` | `docker push ghcr.io/keryxlabs/locus-gateway:X.Y.Z` |

### Master Orchestration Script

Use the root-level wrapper to orchestrate component release and image scripts in one place:

```bash
./build.sh --default-version 0.1.0
```

Common patterns:

```bash
# Release artifacts/checks only
./build.sh --mode release --default-version 0.1.0

# Release artifacts/checks and publish outputs to GitHub/crates.io targets
./build.sh --mode release --default-version 0.1.0 --publish

# Build and tag only service images (mcp + gateway)
./build.sh --mode images --stack services --default-version 0.1.0
```

### Suggested Release Sequence

```bash
cargo check --workspace
cargo test --workspace

# Optional one-command orchestrated release/images flow
./build.sh --default-version 0.1.0

./locus-core/publish-crates.sh
cargo publish --manifest-path locus-sdk/Cargo.toml --dry-run

./locus-mcp/build.sh --publish
./locus-gateway/build.sh --publish

git tag locus-core/v0.2.0
git tag locus-sdk/v0.1.0
git tag locus-mcp/v0.1.0
git tag locus-gateway/v2.0.0
git tag locus-cli/v0.1.0
git push origin locus-core/v0.2.0 locus-sdk/v0.1.0 locus-mcp/v0.1.0 locus-gateway/v2.0.0 locus-cli/v0.1.0

./locus-mcp/build-image.sh ghcr.io/keryxlabs/locus-mcp:0.1.0
docker push ghcr.io/keryxlabs/locus-mcp:0.1.0

./locus-gateway/build-image.sh ghcr.io/keryxlabs/locus-gateway:2.0.0
docker push ghcr.io/keryxlabs/locus-gateway:2.0.0
```

## Operational Guardrails

Production-facing guidance:

1. Keep provider endpoints, credentials, and tokens externalized.
2. Use explicit tenant/session scoping in all host integrations.
3. Keep retrieval fallback policy explicit, not implicit.
4. Validate parser and validator strict-profile compatibility on generated nodes.
5. Run dry-run mutation paths before applying large transforms.

Release readiness checks:

1. Workspace compile and tests pass.
2. SDK examples compile/run in CI smoke path.
3. Host compatibility checks pass for MCP and Gateway contracts.
4. Changelog and migration notes are updated.
5. Version policy review is complete for the target release line.

## Repository Layout

```text
locus/
	locus-core/      # domain contracts, parser/validator, storage, retrieval, sync-ready mechanics
	locus-sdk/       # primitives, composition workflows, provider adapters/registry
	locus-mcp/       # stdio MCP host
	locus-gateway/   # HTTP + gRPC host
	locus-cli/       # operator-facing SDK-backed CLI for memory workflows
	docs/            # architecture, deployment, integration, operations, examples, security
```

## Documentation Map (Detailed)

Use docs by concern rather than reading in strict order:

1. [docs/architecture.md](docs/architecture.md): boundaries, layering, design intent.
2. [docs/deployment.md](docs/deployment.md): environment profiles and deployment guidance.
3. [docs/operations.md](docs/operations.md): runtime operations and maintenance practices.
4. [docs/integration.md](docs/integration.md): migration and contract compatibility guidance.
5. [docs/examples.md](docs/examples.md): runnable SDK examples and expected coverage.
6. [docs/troubleshooting.md](docs/troubleshooting.md): failure-mode triage and recovery.
7. [docs/versioning.md](docs/versioning.md): SemVer and compatibility policy.
8. [docs/security.md](docs/security.md): security posture and handling discipline.
9. [docs/sttp_typed_ir_language_spec.md](docs/sttp_typed_ir_language_spec.md): typed IR protocol reference.

## Why The Repository Is Structured This Way

One architectural boundary is intentional and enforced:

1. Core and SDK own reusable memory behavior.
2. Hosts own transport, deployment, and policy.

This allows teams to adopt a minimal surface first and grow into broader deployment shapes without rewriting memory logic.

## Release Notes And Change History

Crate-level release notes:

1. [locus-core/CHANGELOG.md](locus-core/CHANGELOG.md)
2. [locus-gateway/CHANGELOG.md](locus-gateway/CHANGELOG.md)

## Contributing

Contributions are welcome across crates, docs, and operational tooling.

See [CONTRIBUTING.md](CONTRIBUTING.md) for workflow and validation expectations.

When making changes:

1. Keep external contracts stable unless a planned versioned break is required.
2. Prefer additive behavior changes first.
3. Include migration notes for any contract-impacting changes.
4. Add tests for behavior changes in retrieval, parsing, or transforms.

## Community And Governance

For public collaboration standards and disclosure policy:

1. [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
2. [CONTRIBUTING.md](CONTRIBUTING.md)
3. [SECURITY.md](SECURITY.md)
4. [LICENSE](LICENSE)
