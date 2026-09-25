# locus-sdk Changelog

All notable changes specific to locus-sdk are documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- **Reactive memory reflex (schema v4)**: `MemoryReflexService` turns a stimulus into a bus envelope by attaching a System 1 decider. The decider answers `choice` / `score` / `noul` questions (the Laya and Jev `/v1/systemone` contract). The gate dispatches recall, find, persist, explain, or aggregate only when confidence, salience, and the propositions agree. Otherwise the envelope is `ignore` or `escalate` and carries no runnable request.
- `HttpSystem1` posts that catalog to a Laya, `sys1`, or Jev-compatible endpoint. `HeuristicSystem1` answers the same questions offline. `request_for` + `apply` accepts a forward pass the host already ran.
- The service does not subscribe, publish, or open a store. `topic` names (`locus.memory.*`) are for the host's own bus. Schema introspection adds `reflex_actions` and `decision_types`.

## [0.4.0] - 2026-09-22

### Added

- Natural-language `query_text` on recall and explain. A multi-word question is matched by content-term overlap (with light inflection) against `context_summary`, semantic tags, and raw node text, over a scoped scan of the newest 2000 nodes. Hits replace pure resonance ranking. When a query embedding is present, hits are prepended to the hybrid ranking instead of discarding it.
- `strictness` now controls how many content terms must hit: precision requires all of them, balanced requires half, recall requires one.

### Changed

- Single-token `query_text` is unchanged: exact phrase fallback still runs only for `on_empty` when the primary set is empty, or for `always`.
- Still depends on `locus-core-rs` 0.5.1. Core and `locus-surreal-adapter` are not part of this release.

### Parity

- `fallback_policy=never` does not run the natural-language scan. Ranking stays resonance or hybrid.
- Single-token `query_text` with `on_empty` still exact-matches only when the primary set is empty. `always` still merges that exact match into a non-empty primary set.
- Multi-word `query_text` with `on_empty` or `always` ranks content-term hits ahead of pure AVEC resonance. Previously the full question had to appear as one substring, and only when fallback ran.
- When `query_embedding` is set, term hits are prepended and the hybrid tail is kept.
- Validation: `cargo test -p locus-sdk --lib` covers question ranking against resonance, single-token non-override, explain fallback, and strictness.

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
