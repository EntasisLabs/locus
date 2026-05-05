# Troubleshooting

## Triage Workflow

Use this sequence to reduce time-to-diagnosis:

1. Reproduce with the smallest failing input.
2. Classify failure domain: build/test, parser/validator, retrieval, or transform.
3. Run the command pack for that domain.
4. Capture escalation bundle before changing runtime defaults.

## Command Packs

### Build and Test Command Pack

Run from repository root:

```bash
cargo check --workspace --examples
cargo test --workspace
cargo test -p locus-core --lib
cargo test -p locus-sdk
```

Use when:

1. CI fails but local state is unclear.
2. A release candidate needs a fast baseline confidence pass.

### Parser and Validator Command Pack

Run from repository root:

```bash
cargo test -p locus-core parser_tests
cargo test -p locus-core validator_tests
cargo test -p locus-core end_to_end_parsing_tests
```

Use when:

1. Ingestion rejects node payloads.
2. Strict profile compatibility appears unstable.

### Retrieval Command Pack

Run from repository root:

```bash
cargo test -p locus-core context_query_service_tests
cargo test -p locus-sdk --example memory_composition
cargo test -p locus-sdk --example provider_registry_setup
```

Use when:

1. retrieval_path behavior diverges from expected baseline.
2. recall and explain outputs are not aligned.

### Transform and Sync Command Pack

Run from repository root:

```bash
cargo test -p locus-core monthly_rollup_service_tests
cargo test -p locus-core sync_coordinator_service_tests
cargo test -p locus-core surrealdb_node_store_tests
cargo test -p locus-core surrealdb_runtime_tests
```

Use when:

1. Backfill, rollup, or sync progression stalls.
2. selected/updated/skipped counts are inconsistent.

## Build and Test Failures

### Build/Test Symptom

`cargo test` fails in integration targets while unit tests pass.

### Build/Test Actions

1. Run crate-scoped tests first to isolate failure domain.
2. Run failing target with single thread when test state leakage is suspected.
3. Compare behavior against known baseline in source repository if migrating.
4. Capture failing command output and the exact crate version set.

### Build/Test Common Signatures

1. Integration tests fail while crate lib tests pass: likely environment or fixture state isolation issue.
2. Workspace build passes but host runtime fails: likely endpoint/config mismatch.

## Parser and Validation Errors

### Parser/Validation Symptom

Stored node rejected or strict parser fails.

### Parser/Validation Actions

1. Confirm four-layer ordering is preserved.
2. Confirm content keys use field_name(.confidence) format.
3. Confirm content nesting depth does not exceed five.
4. Re-run parser and validator command pack and compare strict versus tolerant outcomes.

### Parser/Validation Common Signatures

1. Missing required layer errors indicate malformed STTP spine.
2. Invalid key format errors indicate content keys not using field_name(.confidence).
3. Nesting depth errors indicate recursive payload exceeded protocol constraints.

## Retrieval Behavior Mismatch

### Retrieval Symptom

Unexpected recall ranking or fallback path.

### Retrieval Actions

1. Log full scoring policy and scope.
2. Verify alpha and beta values are explicitly set.
3. Use explain workflow to inspect fallback triggers and stage counts.
4. Compare retrieval_path distribution to recent baseline window before changing policy defaults.

### Retrieval Common Signatures

1. Unexpected lexical_fallback path often indicates sparse embeddings or strict filter scope.
2. Score drift with unchanged input usually indicates policy defaults changed between versions.

## Transform/Migration Issues

### Transform/Migration Symptom

Embedding backfill updates fewer nodes than expected.

### Transform/Migration Actions

1. Run dry-run first and inspect selected counts.
2. Validate filter scope and has_embedding settings.
3. Validate provider capability routing and model configuration.
4. Verify checkpoint progression and sync cursor ordering when running incremental jobs.

### Transform/Migration Common Signatures

1. Selected count near zero in dry-run usually indicates filter constraints are too narrow.
2. Update counts diverge from selected counts when provider or storage writes are failing.

## Escalation Data To Capture

1. Exact request payload.
2. Retrieval path and fallback reason.
3. Validation and parser error text.
4. Relevant crate versions and commit sha.
5. Environment profile and runtime mode used during failure.

## Rollback Triggers

Initiate rollback when any of the following occur:

1. Client-visible contract mismatch appears in migrated host routes.
2. Error rate or latency breaches remain sustained after initial mitigation window.
3. Parser or validator strict failures correlate with the latest release.

## Escalation Bundle Template

Capture the following in one incident record:

1. Failing command and timestamp.
2. Environment profile: local, CI, staging, or production.
3. Request payload and redacted response sample.
4. retrieval_path or transform result counters.
5. Recent change references and suspected blast radius.

This bundle enables rapid handoff across development, platform, and on-call teams.
