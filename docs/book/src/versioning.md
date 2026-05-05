# Versioning and Compatibility

## Scope

This policy defines versioning expectations for Locus crates and host integrations.

It also defines dependency governance requirements used to keep releases stable over time.

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

## Dependency Governance

### Dependency Update Cadence

1. Security updates: prioritized and released on an expedited path.
2. Patch and minor dependency updates: batched on regular release cadence.
3. Major dependency updates: planned with explicit compatibility validation and rollback criteria.

### Dependency Change Controls

1. Dependency changes must include rationale and impact scope in release notes.
2. Runtime-facing dependencies require integration smoke checks before release.
3. Changes affecting transport, storage, or crypto behavior require targeted regression coverage.

### Security Patch Handling

1. Vulnerabilities with active exposure are patched on the nearest release path.
2. Mitigations and operational impact are documented with the release.
3. Consumers receive upgrade guidance when patch behavior changes are observable.

### Compatibility Validation for Dependency Changes

1. Workspace compile and tests must pass.
2. Host runtime smoke checks must pass for MCP and gateway.
3. SDK example workflows must pass for reference integration patterns.
4. Generated technical docs must build without warnings requiring manual interpretation.

### SemVer Range Management Policy

1. Internal workspace crate dependencies should use intentionally aligned versions at release time.
2. External dependencies should avoid overly broad ranges that mask breaking behavior.
3. Major version upgrades require explicit compatibility notes and rollout strategy.
4. Lockfile and resolved-version deltas should be reviewed with dependency change rationale.
