# locus-wasm Changelog

All notable changes to `locus-wasm` are documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.1.1] - 2026-08-25

### Changed

- `version()` DTO now reports `core` 0.5.1 and `sdk` 0.3.1.
- Dependency alignment: `locus-core-rs` 0.5.1, `locus-sdk` 0.3.1, `locus-surreal-adapter` 0.1.1.

## [0.1.0] - 2026-08-22

### Added

- Initial **`wasm-bindgen` cdylib** exposing Locus SDK capabilities to browsers.
- Sync exports: `parse_sttp`, `validate_sttp`, `memory_schema`, `compress_text`, `version`.
- **`WasmLocusClient`** with in-memory `store`, `find`, and `recall` workflows.
- Surreal backends (`surreal` feature):
  - `connect_surreal_client` — generic endpoint connect
  - `connect_indxdb_client` — persistent IndexedDB via `indxdb://`
  - `connect_mem_surreal_client` — embedded `mem://` engine
  - WebSocket remote via `wss://` / `ws://` endpoints
- `locus-web` integration: TypeScript facade, `npm run build:wasm`, Vitest smoke test.

### Dependencies

- `locus-core-rs` 0.5.0, `locus-sdk` 0.3.0 (no default features), `locus-surreal-adapter` 0.1.0 (`wasm` profile).
