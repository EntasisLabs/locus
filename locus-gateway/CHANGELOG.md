# locus-gateway Changelog

All notable changes specific to locus-gateway are documented in this file.
For historical entries before this split, see ../CHANGELOG.md.

## [Unreleased]

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
- Preserved behavior while modularizing:
	- HTTP/gRPC route surface and compatibility aliases remain unchanged
	- default and `candle-local` test paths remained green through the refactor
- Added embedding-focused retrieval endpoint for hybrid RAG + AVEC vector queries:
	- `POST /api/v1/context/embeddings`
	- aliases: `POST /api/context/embeddings`, `POST /context/embeddings`
	- accepts separate RAG and AVEC embeddings (or query text), fuses with configurable weights, then executes hybrid context retrieval
	- validates dimension mismatches and returns `400` for invalid embedding combinations
- Added gRPC parity for embedding-focused retrieval:
	- `GetEmbeddingContext(GetEmbeddingContextRequest) -> GetContextReply`
	- supports separate RAG and AVEC embeddings/text with weighted fusion before hybrid retrieval
	- returns `INVALID_ARGUMENT` for invalid embedding combinations (for example, mismatched dimensions)

- Added Resonantia BYO Node Store compatibility aliases for HTTP endpoints:
	- `POST /api/store`, `POST /store` -> `POST /api/v1/store`
	- `GET /api/nodes`, `GET /nodes` -> `GET /api/v1/nodes`
	- `GET /api/graph`, `GET /graph` -> `GET /api/v1/graph`
- Added BYO CORS support and preflight handling with permissive defaults.
- Added tenant header aliases for HTTP resolution (`x-resonantia-tenant`, `x-tenant-id`, `x-tenant`).
- Updated Node Store HTTP response compatibility:
	- list nodes now includes `syncKey` and `syntheticId`
	- store response now includes `duplicateSkipped` and `upsertStatus`
- Added BYO session rename endpoint support:
	- `POST /api/v1/session/rename`
	- aliases: `POST /api/session/rename`, `POST /session/rename`
	- request fields: `sourceSessionId`, `targetSessionId`, `allowMerge`
	- response fields: `sourceSessionId`, `targetSessionId`, `movedNodes`, `movedCalibrations`, `scopesApplied`

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
