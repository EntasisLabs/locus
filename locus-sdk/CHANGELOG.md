# locus-sdk Changelog

All notable changes specific to locus-sdk are documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

## [0.3.1] - 2026-08-25

### Changed

- Recursive composite example and conformance test now assemble full nodes via `SttpDocumentBuilder` (shallow content merge + canonical render) instead of ad-hoc string templates.
- Dependency alignment: `locus-core-rs` 0.5.1.

## [0.3.0] - 2026-08-22

### Added

- **WASM compilation profile**: build with `--no-default-features` for `wasm32-unknown-unknown`.
- **`http-providers` feature** gating `reqwest` and `OllamaEmbeddingProvider`.
- **`testing` feature** gating `faker` / `rand` test helpers and the `generate_faker_fixture` example.
- Target-specific `getrandom` dependencies for WASM RNG (`js` / `wasm_js`).

### Changed

- Default features remain `genai-provider` + `http-providers` for native consumers.
- Dependency alignment: `locus-core-rs` 0.5.0.

## [0.2.2] - 2026-06-23

### Fixed

- Aligned with `locus-core-rs` **0.4.2**: semantic tag/link null handling in parser and SurrealDB storage no longer fails ingest or node reads when `semantic_tags` / `semantic_links` are absent or explicitly `null`.

### Changed

- Dependency alignment: `locus-core-rs` 0.4.2.

## [0.2.1] - 2026-06-23

### Fixed

- Aligned with `locus-core-rs` **0.4.1**: ingest paths that sync `semantic_tag_index` now use the canonical upsert sync key, restoring reliable `indexed_tags` pre-filter behavior for SDK find/recall/graph flows.

### Changed

- Dependency alignment: `locus-core-rs` 0.4.1.

## [0.2.0] - 2026-06-23

### Added

- **Semantic memory primitives (schema v2)**:
  - `MemoryGraphService` — materializes session topology, lineage, and semantic link edges at read time.
  - Extended `MemoryFilter` with `indexed_tags`, `tag_prefix`, `has_semantic_links`, `link_rel`, `link_target`, `links_to_ref`, and related predicates via `memory_filters`.
  - `MemoryScoring.gamma` and tag-embedding fusion path in hybrid recall.
  - Transform operations: `embed_tag_backfill`, `reindex_tag_embeddings` on `semantic_tag_index` rows.
- **Node eviction primitives (schema v3)**:
  - `MemoryEvictService` with modes `by_sync_keys`, `by_node_ids`, `by_filter`, and `purge_session`.
  - Reference safety: blocks delete when inbound `parent_node_id` or `ref:` semantic links exist unless `force=true`.
  - Dry-run preview, semantic tag index cleanup on delete, and session purge with optional calibration/checkpoint removal.
- Domain modules: `domain/graph.rs`, `domain/evict.rs`.
- `MemoryEvictService`, `MemoryGraphService`, and filter helpers wired with optional `SemanticIndexStore`.

### Changed

- Memory schema version bumped to **`locus-sdk.memory.v3`** (`evict_operations`: `delete_nodes`, `purge_session`).
- `MemoryFindService` and `MemoryRecallService` integrate indexed tag pre-filtering when a semantic index is configured.
- `MemoryTransformService` supports tag-index backfill and reindex flows.

### Tests

- Unit tests for `MemoryGraphService`, `MemoryEvictService` (blocking, force, dry-run, filter mode, tag cleanup, session purge), and updated schema/composition assertions.

## [0.1.2] - 2026-05-09

### Changed

- Aligned with `locus-core-rs` 0.3.0 parse-profile and store contract updates.
