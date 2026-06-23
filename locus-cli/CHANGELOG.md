# locus-cli Changelog

All notable changes specific to locus-cli are documented in this file.

## [0.2.1] - 2026-06-23

### Fixed

- Rebuilt against `locus-core-rs` **0.4.1** so `store` / rollup ingest aligns tag-index sync keys with persisted nodes (same fix as gateway/MCP for `indexed_tags` retrieval).

### Changed

- Dependency alignment: `locus-core-rs` 0.4.1, `locus-sdk` 0.2.1.

## [0.2.0] - 2026-06-23

### Added

- **`locus graph`** — SDK-backed memory graph export with session scope, tag filters, and link filters.
- **`locus evict`** — explicit node deletion with `--sync-key`, `--tags`, `--dry-run`, `--force`, and `--purge-session` (plus optional calibration/checkpoint cleanup on purge).

### Changed

- `Nodes` and `Context` commands accept semantic tag and link filter flags aligned with gateway/MCP transport parity.
- Wired `MemoryGraphService` and `MemoryEvictService` with semantic index support.

## [0.1.0] - 2026-05-09

### Added

- Initial SDK-backed CLI for calibrate, store, context, nodes, moods, and monthly rollup against in-memory or SurrealDB backends.
