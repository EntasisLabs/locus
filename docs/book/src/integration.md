# Integration

## Audience and Use

This page is for developers integrating Locus into applications, services, and agent tooling.

Use it to:

1. Define a stable integration boundary.
2. Migrate existing retrieval logic without contract breakage.
3. Validate behavior parity before cutover.

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

This policy enables staged migrations while preserving downstream client compatibility.

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

Migration is complete only when all acceptance criteria are met.

## Integration Acceptance Templates

### Contract Parity Template

1. Endpoint or tool name unchanged.
2. Request field semantics unchanged or additive.
3. Response field semantics unchanged or additive.
4. Error code and message class behavior unchanged.

### Retrieval Behavior Template

1. retrieval_path distribution aligned with baseline behavior.
2. Explain output aligns with returned recall results.
3. Fallback activation remains policy-consistent.
4. Latency profile remains within agreed operational bounds.

### Rollout Gate Template

1. Local and CI validation complete.
2. Staging parity checks complete.
3. Rollback path validated.
4. Monitoring and alert hooks confirmed in target environment.

## Cutover Checklist Template

### Pre-Cutover Checks

1. Freeze incompatible contract changes during the cutover window.
2. Confirm request and response snapshots for baseline comparison.
3. Confirm on-call ownership and escalation path.

### Parity Checks During Cutover

1. Compare retrieval_path and explain behavior between old and new integration paths.
2. Compare latency and error rate against baseline windows.
3. Validate parser and validator success ratios remain stable.

### Rollback Criteria

1. Error-rate increase exceeds agreed threshold.
2. Contract mismatch is observed in client-visible payloads.
3. Retrieval behavior diverges from baseline without approved policy change.

### Post-Cutover Verification

1. Re-run acceptance templates in production mode.
2. Confirm no delayed transform or ingestion side effects.
3. Publish cutover summary with observed deltas and follow-up actions.
