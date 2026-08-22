# locus-surreal-adapter Changelog

All notable changes to `locus-surreal-adapter` are documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.1.0] - 2026-08-22

### Added

- Initial release of shared **`RuntimeSurrealDbClient`** implementing [`locus_core_rs::SurrealDbClient`].
- **`native` feature** (default for hosts): `protocol-http`, `kv-surrealkv`, `kv-mem`, `rustls`, and tracing-backed query logs.
- **`wasm` feature** for browser/WASM consumers: `kv-indxdb`, `kv-mem`, and `protocol-ws`.
- Endpoint helpers (`is_embedded_endpoint`, `is_remote_endpoint`, `effective_use_remote`) for `indxdb://`, `mem://`, and WebSocket URLs.

### Changed

- Deduplicated Surreal client implementations previously copied across `locus-cli`, `locus-mcp`, and `locus-gateway`.
