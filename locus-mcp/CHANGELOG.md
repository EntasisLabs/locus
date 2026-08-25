# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.3.1] - 2026-08-25

### Changed

- Dependency alignment: `locus-core-rs` 0.5.1, `locus-sdk` 0.3.1, `locus-surreal-adapter` 0.1.1.

## [0.3.0] - 2026-08-22

### Changed

- SurrealDB client wiring moved to shared **`locus-surreal-adapter`** crate.
- Dependency alignment: `locus-core-rs` 0.5.0, `locus-sdk` 0.3.0, `locus-surreal-adapter` 0.1.0.

## [0.2.2] - 2026-06-23

### Fixed

- Rebuilt against `locus-core-rs` **0.4.2** so `store_context` and node query tools handle `semantic_tags: null` / `semantic_links: null` in STTP payloads and deserialize legacy Surreal rows with null semantic columns.

### Changed

- Dependency alignment: `locus-core-rs` 0.4.2, `locus-sdk` 0.2.2.

## [0.2.1] - 2026-06-23

### Fixed

- Rebuilt against `locus-core-rs` **0.4.1** so `store_context` tag-index sync matches persisted node sync keys — fixes `indexed_tags` pre-filter misses when using MCP semantic tag filters after ingest.

### Changed

- Dependency alignment: `locus-core-rs` 0.4.1, `locus-sdk` 0.2.1.

## [0.2.0] - 2026-06-23

### Added

- **`get_graph` MCP tool** — session-scoped memory graph with semantic tag and link filters.
- **`evict_nodes` MCP tool** — explicit delete by sync key, node id, semantic filter, or session purge; supports `dry_run`, `force`, and purge calibration/checkpoint options.
- Semantic filter parameters on `get_context`, `list_nodes`, and related tools (`semantic_tags`, `link_rel`, `link_target`, `links_to_ref`, `tag_prefix`, `has_semantic_links`, `gamma`).
- `composition.rs` and per-tool modules under `src/tools/` for single-responsibility MCP handlers.

### Changed

- Refactored server startup into a clean composition architecture (split from monolithic `main.rs`).
- Strict typed-IR schema-first guidance in `store_context` error payloads.
- Parse profile configuration via `LOCUS_MCP_PARSE_PROFILE` and `--parse-profile`.
- Dependency alignment: `locus-core-rs` 0.4.0, `locus-sdk` 0.2.0.
- Crate version bumped to `0.2.0`.

## [0.1.2] - 2026-05-09

### Added
- Strict typed-IR schema-first guidance in `store_context` error payloads, including model-facing recovery steps.
- Explicit parser profile configuration via `LOCUS_MCP_PARSE_PROFILE` and `--parse-profile`.
- Clear VS Code MCP setup examples that show parse-profile configuration for both strict and tolerant onboarding flows.

### Changed
- Refactored server startup into a clean composition architecture by splitting responsibilities from `src/main.rs` into:
  - `src/composition.rs`
  - `src/shared.rs`
  - `src/tools/*` (one MCP tool implementation per file)
- Updated `StoreContextService` wiring to use explicit parser injection at composition time.
- Updated dependency versions to align with current workspace/runtime behavior:
  - `locus-core-rs` -> `0.3.0`
  - `locus-sdk` -> `0.1.2`

### Fixed
- Improved strict-mode rejection behavior so invalid typed-IR payloads fail with explicit parse/policy diagnostics.
- Verified strict-mode end-to-end behavior after restart:
  - invalid payloads are rejected
  - valid payloads are accepted and retrievable

## [0.1.1] - 2026-05-08

### Changed
- Retroactive release note: this was the first `locus-mcp` changelog-worthy patch release, but no changelog file existed at the time.
- Updated published dependency wiring to avoid local-path-only dependency assumptions in release artifacts.
- Bumped MCP crate version to `0.1.1` and published tag `locus-mcp/v0.1.1`.
