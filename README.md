# Locus

Typed memory for agents: store context, recall it later, and explain continuity across sessions.

Locus is a set of composable Rust crates, not a UI product. The same memory behavior is available in-process ([locus-sdk](locus-sdk/README.md)), over stdio ([locus-mcp](locus-mcp/README.md)), over HTTP and gRPC ([locus-gateway](locus-gateway/README.md)), and from a terminal ([locus-cli](locus-cli/README.md)).

**STTP** is the typed intermediate representation each memory node is written in: four ordered layers (provenance, envelope, content, metrics) that are parsed and validated before storage. **AVEC** is the state vector on that node (`stability`, `friction`, `logic`, `autonomy`, and `psi`, their sum). Recall ranks stored nodes against the caller's current AVEC, with optional semantic signals. Protocol spec: [docs/sttp_typed_ir_language_spec.md](docs/sttp_typed_ir_language_spec.md).

Licensed under [Apache-2.0](LICENSE).

## Install and run

Tags match crate versions in `Cargo.toml` on this branch. Replace a tag when you pin a different release. Mount a volume when host state should survive the container, and keep dev and production data paths separate.

### MCP image — `locus-mcp` 0.4.1

```bash
docker run --rm -i -v "$PWD/locus-data:/data" ghcr.io/entasislabs/locus-mcp:0.4.1
```

### Gateway image — `locus-gateway` 0.5.1

```bash
docker run --rm -p 8080:8080 -p 8081:8081 -v "$PWD/locus-data:/data" ghcr.io/entasislabs/locus-gateway:0.5.1
```

HTTP listens on `8080`. gRPC listens on `8081`.

### SDK — `locus-sdk` 0.5.0

Published on crates.io:

```toml
locus-sdk = "0.5.0"
```

From this repository, run the System 1 envelope example:

```bash
cargo run -p locus-sdk --example memory_reflex
```

### CLI — `locus-cli` 0.4.1

From this repository:

```bash
cargo run -p locus-cli -- --help
```

Release archives for MCP, Gateway, and CLI are built with the component `build.sh` scripts. Commands are in [Build, tags, and release](#build-tags-and-release-process).

## What ships

Workspace crates (versions from `Cargo.toml`):

1. [locus-core-rs](locus-core-rs/README.md) `0.5.1`: parser/validator, domain contracts, storage abstractions, retrieval services, sync-ready mechanics.
2. [locus-sdk](locus-sdk/README.md) `0.5.0`: primitive-first SDK, composition workflows, and `MemoryReflex`.
3. [locus-surreal-adapter](locus-surreal-adapter) `0.1.1`: SurrealDB runtime client for hosts and WASM consumers.
4. [locus-mcp](locus-mcp/README.md) `0.4.1`: stdio MCP server exposing memory tools.
5. [locus-gateway](locus-gateway/README.md) `0.5.1`: deployable Rust gateway with HTTP + gRPC.
6. [locus-cli](locus-cli/README.md) `0.4.1`: operator-facing SDK-backed CLI for common memory workflows.
7. [locus-wasm](locus-wasm) `0.1.3`: WebAssembly bindings for STTP parsing and in-browser memory. [locus-web](locus-web) builds that bundle (`npm run build:wasm` from `locus-web`). `locus-wasm` is git-tagged, not published to crates.io.

**MemoryReflex** (`locus-sdk` 0.5.0) is the System 1 bus envelope. `MemoryReflexService` takes a `MemoryStimulus` and an attached decider. The decider answers one `choice` (`action`), one `score` (`salience`), and three `noul` propositions (`references_prior`, `should_persist`, `needs_system2`) in a single pass. The service returns a `MemoryReflex` envelope for the host to publish. Recall, find, aggregate, and persist payloads are filled only when the gate accepts the decision. `Ignore` and `Escalate` carry no runnable request.

```bash
cargo run -p locus-sdk --example memory_reflex
```

CLI Skills are not in this repository. MCP is the entry point when you do not want to write service code.

## Pick a path

| Goal | Start |
| --- | --- |
| Memory tools in an MCP client | [locus-mcp/README.md](locus-mcp/README.md), or the image command above |
| In-process Rust API | [locus-sdk/README.md](locus-sdk/README.md) and [locus-core-rs/README.md](locus-core-rs/README.md) |
| HTTP or gRPC host for apps and services | [locus-gateway/README.md](locus-gateway/README.md) |
| Terminal workflows | [locus-cli/README.md](locus-cli/README.md) |
| Release policy and operations | [docs/versioning.md](docs/versioning.md), [docs/deployment.md](docs/deployment.md), [docs/operations.md](docs/operations.md) |

## Support matrix

| Area | Baseline |
| --- | --- |
| Rust toolchain | Stable |
| Cargo | Included with stable toolchain |
| Host OS | Linux, macOS, Windows (WSL2 recommended) |
| Container runtime | Docker Engine or compatible |
| Storage modes | In-memory and SurrealDB v3 |
| Docs tooling | mdbook and mdbook-mermaid |

Published images first, then release binaries, then source builds.

## From this repository

```bash
cargo check --workspace
cargo test --workspace
cargo check --examples -p locus-sdk
```

MCP multi-target archives:

```bash
./locus-mcp/build.sh
```

Gateway multi-target archives:

```bash
./locus-gateway/build.sh
```

Both scripts support `--publish` to upload packaged artifacts to the matching namespaced GitHub release tag.

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
cargo run -p locus-sdk --example memory_reflex
cargo run -p locus-sdk --example provider_registry_setup
cargo run -p locus-sdk --example memory_composition
cargo run -p locus-sdk --example recursive_composite_pipeline
```

Run CLI help:

```bash
cargo run -p locus-cli -- --help
```

## SDK, MCP, and Gateway

### SDK usage summary

Use [locus-sdk/README.md](locus-sdk/README.md) when you want transport-agnostic memory behavior in-process.

Core primitives:

1. `memory_find`: deterministic filtering/sorting.
2. `memory_recall`: ranked retrieval using AVEC and optional semantic signals.
3. `memory_aggregate`: grouped stats and rollups.
4. `memory_transform`: controlled mutation/backfill workflows.
5. `memory_explain`: visibility into recall decisions.
6. `memory_schema`: runtime capability introspection.
7. `memory_reflex`: System 1 bus envelope (`MemoryReflex`) from a stimulus. The service does not open a store.

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
5. Use `memory_reflex` when a host should decide the operation before calling a primitive.

### MCP usage summary

Use [locus-mcp/README.md](locus-mcp/README.md) when memory should be exposed through stdio tools to assistants and agents.

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

### Gateway usage summary

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

## End-to-end examples

### Gateway health, calibrate, store, context

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

### SDK example execution

```bash
cargo run -p locus-sdk --example memory_composition
```

What this gives you:

1. Recall with explain output.
2. Daily rollup sample.
3. Capability bundle output.
4. Transform-then-recall verification path.

```bash
cargo run -p locus-sdk --example memory_reflex
```

What this gives you: sample stimuli turned into `MemoryReflex` envelopes and printed from a local channel. Recall and persist fields are present only on `Dispatch`.

### MCP example operational flow

For an MCP-capable assistant client:

1. Call `calibrate_session` with current AVEC state.
2. Call `store_context` with one valid STTP node.
3. Call `get_context` with the same session and AVEC state.
4. Call `list_nodes` when inventory review is needed.
5. Call `create_monthly_rollup` for timeline compaction.

## Technical docs site

Build the mdBook guide and workspace rustdoc from the repo root:

```bash
./docs/build-technical-docs.sh
```

Generated output:

1. `docs/technical/book/index.html` (mdBook guide set)
2. `docs/technical/api/index.html` (workspace rustdoc)
3. `docs/technical/index.html` (combined entrypoint)

Recommended first technical pages:

1. `docs/book/src/environment-setup.md` (tooling and environment baseline)
2. `docs/book/src/deployment.md` (runtime profiles and release readiness)
3. `docs/book/src/integration.md` (contract-safe migration and rollout gates)

Requirements:

1. `mdbook` installed (`cargo install mdbook`)
2. `mdbook-mermaid` installed (`cargo install mdbook-mermaid`)
3. Standard Rust toolchain for `cargo doc`

## Operational guardrails

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

## Repository layout

```text
locus/
	locus-core-rs/           # domain contracts, parser/validator, storage, retrieval, sync-ready mechanics
	locus-sdk/               # primitives, composition workflows, MemoryReflex, provider adapters
	locus-surreal-adapter/   # SurrealDB runtime client for hosts and WASM
	locus-wasm/              # WASM bindings for STTP parsing and in-browser memory
	locus-mcp/               # stdio MCP host
	locus-gateway/           # HTTP + gRPC host
	locus-cli/               # operator-facing SDK-backed CLI
	locus-web/               # browser app; builds the locus-wasm bundle
	docs/                    # architecture, deployment, integration, operations, examples, security
```

## Documentation map

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
10. [docs/sttp_document_builder.md](docs/sttp_document_builder.md): fluent canonical node construction (shallow content merge).
11. [docs/sdk-architecture.md](docs/sdk-architecture.md): SDK layering, including `MemoryReflexService`.

Index: [docs/README.md](docs/README.md).

## Why the repository is structured this way

One architectural boundary is intentional and enforced:

1. Core and SDK own reusable memory behavior.
2. Hosts own transport, deployment, and policy.

This allows teams to adopt a minimal surface first and grow into broader deployment shapes without rewriting memory logic.

## Build, tags, and release process

Version policy: [docs/versioning.md](docs/versioning.md). Locus uses Instrumenta-style namespaced component release lines. The matrix and commands below are the orchestration this repository runs.

### Tag prefixes

1. `locus-core-rs/v...`
2. `locus-sdk/v...`
3. `locus-mcp/v...`
4. `locus-gateway/v...`

### Artifact matrix

| Component | Artifact Type | Build Command | Publish Action |
| --- | --- | --- | --- |
| `locus-core-rs` | crates.io package | `./locus-core-rs/publish-crates.sh` | add `--publish` for actual crates.io publish |
| `locus-sdk` | crates.io package | `./locus-sdk/publish-crates.sh` | add `--publish` for actual crates.io publish |
| `locus-surreal-adapter` | crates.io package | `./locus-surreal-adapter/publish-crates.sh` | publish after core, before sdk |
| `locus-wasm` | browser WASM bundle | `cd locus-web && npm run build:wasm` | git tag only; not published to crates.io |
| `locus-mcp` | multi-platform archives | `./locus-mcp/build.sh` | `./locus-mcp/build.sh --publish` |
| `locus-gateway` | multi-platform archives | `./locus-gateway/build.sh` | `./locus-gateway/build.sh --publish` |
| `locus-cli` | multi-platform archives | `./locus-cli/build.sh` | `./locus-cli/build.sh --publish` uploads assets to `locus-cli/vX.Y.Z` |
| `locus-mcp` | Docker image | `./locus-mcp/build-image.sh ghcr.io/entasislabs/locus-mcp:X.Y.Z` | `docker push ghcr.io/entasislabs/locus-mcp:X.Y.Z` |
| `locus-gateway` | Docker image | `./locus-gateway/build-image.sh ghcr.io/entasislabs/locus-gateway:X.Y.Z` | `docker push ghcr.io/entasislabs/locus-gateway:X.Y.Z` |

### Master orchestration script

Use the root-level wrapper to orchestrate component release and image scripts in one place:

```bash
./scripts/release.sh              # preflight (check + test)
./scripts/release.sh --build      # preflight + release artifact builds
./scripts/release.sh --build --publish
```

Or invoke `./build.sh` directly with explicit versions:

```bash
./build.sh --mode release \
  --targets core,mcp,gateway,cli \
  --mcp-version 0.4.1 \
  --gateway-version 0.5.1 \
  --cli-version 0.4.1
```

Common patterns:

```bash
# Release artifacts/checks only
./build.sh --mode release --mcp-version 0.4.1 --gateway-version 0.5.1 --cli-version 0.4.1

# Release artifacts/checks and publish outputs to GitHub/crates.io targets
./build.sh --mode release --mcp-version 0.4.1 --gateway-version 0.5.1 --cli-version 0.4.1 --publish

# Build and tag only service images (mcp + gateway)
./build.sh --mode images --stack services --mcp-version 0.4.1 --gateway-version 0.5.1
```

### Suggested release sequence

```bash
# Full preflight + optional builds (see ./scripts/release.sh --help)
./scripts/release.sh
./scripts/release.sh --build

cargo check --workspace
cargo test -p locus-core-rs -p locus-sdk -p locus-gateway -p locus-mcp -p locus-cli

./locus-core-rs/publish-crates.sh --publish
./locus-surreal-adapter/publish-crates.sh --publish
./locus-sdk/publish-crates.sh --publish

./locus-mcp/build.sh --publish
./locus-gateway/build.sh --publish
./locus-cli/build.sh --publish

cd locus-web && npm ci && npm run build:wasm

git tag locus-core-rs/v0.5.1
git tag locus-sdk/v0.5.0
git tag locus-surreal-adapter/v0.1.1
git tag locus-wasm/v0.1.3
git tag locus-mcp/v0.4.1
git tag locus-gateway/v0.5.1
git tag locus-cli/v0.4.1
git push origin \
  locus-core-rs/v0.5.1 \
  locus-sdk/v0.5.0 \
  locus-surreal-adapter/v0.1.1 \
  locus-wasm/v0.1.3 \
  locus-mcp/v0.4.1 \
  locus-gateway/v0.5.1 \
  locus-cli/v0.4.1

./locus-mcp/build-image.sh ghcr.io/entasislabs/locus-mcp:0.4.1
docker push ghcr.io/entasislabs/locus-mcp:0.4.1

./locus-gateway/build-image.sh ghcr.io/entasislabs/locus-gateway:0.5.1
docker push ghcr.io/entasislabs/locus-gateway:0.5.1
```

## Release notes and change history

Crate-level release notes:

1. [locus-core-rs/CHANGELOG.md](locus-core-rs/CHANGELOG.md)
2. [locus-sdk/CHANGELOG.md](locus-sdk/CHANGELOG.md)
3. [locus-gateway/CHANGELOG.md](locus-gateway/CHANGELOG.md)
4. [locus-mcp/CHANGELOG.md](locus-mcp/CHANGELOG.md)
5. [locus-cli/CHANGELOG.md](locus-cli/CHANGELOG.md)

Release helper: [scripts/release.sh](scripts/release.sh)

The name is Latin for "place," from the Method of Loci. That is the name only.

## Contributing

Contributions are welcome across crates, docs, and operational tooling.

See [CONTRIBUTING.md](CONTRIBUTING.md) for workflow and validation expectations.

When making changes:

1. Keep external contracts stable unless a planned versioned break is required.
2. Prefer additive behavior changes first.
3. Include migration notes for any contract-impacting changes.
4. Add tests for behavior changes in retrieval, parsing, or transforms.

## Community and governance

For public collaboration standards and disclosure policy:

1. [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
2. [CONTRIBUTING.md](CONTRIBUTING.md)
3. [SECURITY.md](SECURITY.md)
4. [LICENSE](LICENSE)
