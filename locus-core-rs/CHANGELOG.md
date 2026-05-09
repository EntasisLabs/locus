# locus-core-rs Changelog

All notable changes specific to locus-core-rs are documented in this file.
For historical entries before this split, see ../CHANGELOG.md.

## [0.2.1] - 2026-05-08

### Added

- Introduced `ParseProfile::StrictTypedIr` and wired store ingest to enforce typed-IR required properties at write time.
- Added session-scoped ingest retry controls in `StoreContextService` with cooldown policy defaults (more than 3 consecutive failures triggers a 120s cooldown window).
- Added structured ingest trace events (`[sttp_ingest_trace]`) across validator, parser, and store boundaries with profile, diagnostics, and retry-state context while keeping node content redacted.
- Added structured monthly rollup trace events (`[sttp_rollup_trace]`) for request scope, query results, no-source attribution, and persistence outcomes.

### Changed

- Replaced brittle regex-style strict extraction paths with structural object/key parsing in the STTP parser.
- Tightened strict typed-IR diagnostics to surface actionable reason codes/messages for missing keys, invalid enums, and invalid numerics.
- Corrected strict typed checks for nested prime fields so `provenance.prime.relevant_tier` and `provenance.prime.retrieval_budget` are validated inside the `prime` object, not at provenance top level.

### Tests

- Updated monthly rollup seeding test to assert store acceptance explicitly before rollup execution.
- Added monthly strict-typed regression coverage to ensure no false missing-key diagnostics for nested prime fields.
- Added parser regressions based on production model outputs:
	- JSON-wrapper node payload is rejected under strict typed IR with layer/strict diagnostics.
	- Shorthand layered payload is rejected under strict typed IR when required typed objects/properties are absent.

## [0.2.0] - 2026-05-03

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
