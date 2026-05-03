# locus-core Changelog

All notable changes specific to locus-core are documented in this file.
For historical entries before this split, see ../CHANGELOG.md.

## [Unreleased]

### Added

- ContextQueryService now supports session-optional retrieval for true cross-session memory queries.
- New global retrieval APIs:
	- get_context_global_async(...)
	- get_context_hybrid_global_async(...)
- New scoped optional APIs:
	- get_context_scoped_async(session_id: Option<&str>, ...)
	- get_context_hybrid_scoped_async(session_id: Option<&str>, ...)
- When no session is provided, retrieval now ranks candidates across all sessions using resonance and optional hybrid semantic scoring.

### Changed

- Retrieval resonance scoring now uses full AVEC distance (stability, friction, logic, autonomy) instead of PSI-only distance.
- Hybrid scoring now blends semantic similarity with AVEC resonance (not PSI-only resonance).
- In-memory and SurrealDB store implementations now apply AVEC-first ranking consistently for scoped and global retrieval paths.

### Tests

- Added context_query_service_tests.rs coverage for:
	- mixed-session global retrieval
	- hybrid global retrieval preference by embedding match
	- backward-compatible scoped retrieval behavior

## [0.1.4] - 2026-04-14

### Fixed

- SurrealDB startup backfill now repairs legacy temporal_node rows that are missing persisted sync fields before tenant backfill writes.
- Legacy updated_at fallback order: existing updated_at -> timestamp -> current UTC.
- Legacy sync_key fallback for blank or missing rows: legacy:<node_id>.
- Prevents SCHEMAFULL write failures such as Expected datetime but found NONE when mutating legacy rows.
- Optional connector metadata and source metadata writes now use `NONE`-aware query paths instead of sending `NULL` to `option<object>` fields.

### Changed

- Crate version bumped to `0.1.4`.

## Historical Highlights

- 1.2.1 (2026-04-11): Added sync primitives, typed ConnectorMetadata envelopes, idempotent upserts, cursor-based change queries, and checkpoint persistence.
- 1.2.1 (2026-04-11): Confirmed backward-compatible reads for legacy rows missing sync_key, updated_at, and legacy tenant values.
