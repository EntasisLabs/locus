# Operations

## Observability Priorities

1. Retrieval path visibility for recall workflows.
2. Fallback policy trigger visibility.
3. Transform execution counters and failure reasons.
4. Parser and validator error surface for malformed nodes.

## Runtime Health Signals

1. Parse success ratio.
2. Validation failure ratio.
3. Recall latency percentiles.
4. Transform batch success and failure counts.
5. Embedding backfill throughput and error rates.

## Alert Triggers

1. Sudden rise in validation failures.
2. Recall latency p95 breach sustained over multiple intervals.
3. Unexpected shift in retrieval_path distribution.
4. Transform failure ratio above configured threshold.

## Logging Recommendations

1. Log explicit policy settings for recall and transform requests.
2. Log retrieval path and fallback reason for each recall request.
3. Log session and tenant scope used for each operation.
4. Keep request payload snapshots in redacted form for incident replay.

## Incident Response Playbook

1. Determine whether failure is parser, policy, or storage related.
2. Reproduce with the same request payload and explicit limits.
3. Use explain workflows to isolate ranking or fallback regressions.
4. Run dry-run transforms before any broad mutation in recovery paths.

## Runbooks

### Runbook A: Parser/Validation Spike

1. Capture representative failing payloads.
2. Validate four-layer ordering and content confidence key format.
3. Compare with last known-good release behavior.
4. Roll back host version if failures are release-correlated.

### Runbook B: Retrieval Regression

1. Capture request payload, scoring settings, and retrieval_path.
2. Execute explain workflow with same request.
3. Compare channel scores and fallback behavior against baseline.
4. Patch policy defaults only with regression test coverage.

### Runbook C: Transform Job Instability

1. Switch affected operations to dry-run mode.
2. Validate selected node set and provider capabilities.
3. Reduce batch_size and re-run with checkpoint controls.
4. Resume full run only after zero-failure dry-run parity.

## SLO Suggestions

1. Recall p95 latency target per environment.
2. Parse/validate success target for ingested nodes.
3. Transform job success target and bounded failure budget.

## Maintenance

1. Keep crate versions aligned within workspace releases.
2. Re-run examples as part of regression verification.
3. Audit docs links and command validity each release cycle.
4. Rehearse incident runbooks quarterly.
