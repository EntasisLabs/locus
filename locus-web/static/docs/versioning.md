# Versioning and Compatibility

## Scope

This policy defines versioning expectations for Locus crates and host integrations.

## Package Versioning

1. locus-core and locus-sdk use SemVer.
2. Pre-1.0 versions may evolve rapidly but should still document behavior changes clearly.
3. Breaking changes require explicit migration notes in release documentation.

## Compatibility Rules

1. SDK should remain additive where possible.
2. Core model changes should preserve parse/load compatibility for legacy node rows.
3. Host integrations should keep external endpoint/tool contracts stable during internal rewires.
4. Deprecated contract paths should remain available for at least one release cycle unless security or correctness requires immediate removal.

## Release Channels

1. Patch: bug fixes, non-breaking behavior corrections, docs-only changes.
2. Minor: additive APIs, new optional workflows, new integration hooks.
3. Major: intentional API/contract breaks with migration guide.

## Migration Guarantees

1. Every breaking change requires a migration section with before/after payload examples.
2. Behavior changes in retrieval or fallback policy must include validation evidence and parity notes.
3. Parser and validator contract shifts must include strict-profile regression tests.

## Version Alignment

1. Workspace releases should keep crate versions intentionally aligned when shipped together.
2. If independent crate version bumps are required, release notes must explain compatibility implications.
