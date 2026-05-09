# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

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
