# Integration

## Goal

Integrate host transports with Locus crates while keeping STTP protocol semantics stable.

## Recommended Integration Boundary

1. Parse and validate at ingestion boundary.
2. Convert transport payloads into SDK DTOs.
3. Execute SDK workflows in application services.
4. Map results back to transport contracts.

## Host Migration Pattern

1. Replace transport-owned retrieval logic with locus-sdk primitives.
2. Keep endpoint names stable during migration.
3. Add compatibility tests for behavior parity.
4. Remove deprecated transport-local logic after one release cycle.

## Compatibility Policy

1. Keep external MCP tool and gateway endpoint contracts stable during internal rewires.
2. Prefer additive fields and optional settings over contract replacement.
3. Any unavoidable breaking change requires migration notes and version bump alignment.
4. Retrieval policy defaults must remain explicit and test-covered.

## MCP Integration Notes

1. Prefer SDK primitives for list and context tools.
2. Keep tool contracts stable; change only internals first.
3. Add tests for fallback policy behavior and explain parity.

## Gateway Integration Notes

1. Route HTTP and gRPC handlers into SDK service layer.
2. Share one request mapping strategy across transports.
3. Keep response payload fields stable until formal API revision.

## Safety Checks During Migration

1. Compare retrieval_path results before and after migration.
2. Compare transform counters and side effects in dry-run first.
3. Validate strict parser compatibility for generated nodes.

## Migration Acceptance Criteria

1. Contract parity tests pass for migrated host routes/tools.
2. Explain outputs are consistent with recall behavior before and after migration.
3. Transform counters and dry-run selection remain behaviorally equivalent.
4. Compile and runtime smoke checks pass in workspace and host deployment jobs.
