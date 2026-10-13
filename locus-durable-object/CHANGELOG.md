# locus-durable-object Changelog

All notable changes to the Durable Object store are documented in this file.

## [Unreleased]

## [0.1.0] - 2026-10-13

### Added

- Initial Worker. One Durable Object is one SQLite database, addressed with `getByName(storeName)`.
- Tables match the SQLite adapter: `temporal_node`, `calibration`, `sync_checkpoint`, and `semantic_tag_index`.
- HTTP entry at `GET /health` and `POST /stores/:name`.

This package is git-tagged with the release. It is not published to npm.
