# locus-sqlite-adapter Changelog

All notable changes to `locus-sqlite-adapter` are documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

## [0.1.0] - 2026-10-13

### Added

- Initial release of the SQLite `NodeStore` and `SemanticIndexStore` for file-backed Locus hosts.
- Schema mirrors `temporal_node`, `calibration`, `sync_checkpoint`, and `semantic_tag_index`.

### Changed

- Depends on `locus-core-rs` 0.5.1. Core, `locus-sdk`, and `locus-surreal-adapter` are not part of this release.
