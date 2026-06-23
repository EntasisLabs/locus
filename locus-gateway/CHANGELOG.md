# locus-gateway Changelog

All notable changes specific to locus-gateway are documented in this file.
For historical entries before this split, see ../CHANGELOG.md.

## [0.3.0] - 2026-06-23

### Added

- **Semantic retrieval filters** on context, list-nodes, and graph endpoints: `semanticTags`, `tagsContains`, `linkRel`, `linkTarget`, `linksToRef`, `tagPrefix`, `hasSemanticLinks`, and `gamma` (hybrid tag weight).
- **`GET /api/v1/graph`** (+ aliases `/api/graph`, `/graph`) — memory graph with session topology, lineage, and semantic edges.
- **`POST /api/v1/evict`** (+ aliases `/api/evict`, `/evict`) — explicit node eviction with dry-run, force, filter-based delete, and session purge.
- gRPC/proto fields for `semantic_tags` and `semantic_links` on node payloads.
- App state wiring for `SemanticIndexStore` across find, recall, graph, transform, and evict services.

### Changed

- Refactored gateway structure to reduce `main.rs` responsibilities and improve single-focus iteration:
	- extracted startup/state composition and CORS parsing to `src/orchestration.rs`
	- extracted gateway configuration models to `src/gateway_args.rs`
	- extracted app state wiring to `src/app_state.rs`
	- extracted HTTP request/response DTOs to `src/http_models.rs`
	- extracted embedding + AVEC provider logic to `src/providers.rs`
	- extracted tenant scoping/normalization helpers to `src/tenant.rs`
- Introduced a thin entrypoint design:
	- `src/main.rs` now acts as composition root and delegates runtime execution
	- runtime transport implementation moved to `src/gateway.rs` via `gateway::run()`
- Added embedding-focused retrieval endpoint for hybrid RAG + AVEC vector queries:
	- `POST /api/v1/context/embeddings`
	- aliases: `POST /api/context/embeddings`, `POST /context/embeddings`
- Added gRPC parity for embedding-focused retrieval: `GetEmbeddingContext`.
- Added Resonantia BYO Node Store compatibility aliases for HTTP endpoints.
- Added BYO CORS support and tenant header aliases.
- Updated Node Store HTTP response compatibility (`syncKey`, `syntheticId`, `duplicateSkipped`, `upsertStatus`).
- Added BYO session rename endpoint: `POST /api/v1/session/rename` (+ aliases).
- Dependency alignment: `locus-core-rs` 0.4.0, `locus-sdk` 0.2.0.

### Tests

- HTTP roundtrip coverage for evict dry-run and apply paths.

## [0.2.0] - 2026-06-09

### Added

- Initial gateway release with HTTP/gRPC transport, in-memory and SurrealDB backends, and SDK-backed memory services.

## [1.2.3] - 2026-04-14

### Changed

- Clarified backend selection behavior in documentation and troubleshooting.
- Added explicit guidance that --remote or --surreal-remote-endpoint do not switch the backend by themselves.
- Documented that --backend surreal (or LOCUS_GATEWAY_BACKEND=surreal) is required to read/write SurrealDB data.
- Added explicit query outcome logging in the runtime Surreal client for success/failure visibility across read and mutation operations.
- Crate and default image tag references updated to `1.2.3`.

## Historical Highlights

- 2026-04-12: Documented and validated default backend behavior (`in-memory`) for `/api/v1/nodes` troubleshooting.
- 2026-04-12: Added explicit startup guidance for Surreal mode activation via `--backend surreal` or `LOCUS_GATEWAY_BACKEND=surreal`.
