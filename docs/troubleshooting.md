# Troubleshooting

## Build and Test Failures

### Symptom

`cargo test` fails in integration targets while unit tests pass.

### Actions

1. Run crate-scoped tests first to isolate failure domain.
2. Run failing target with single thread when test state leakage is suspected.
3. Compare behavior against known baseline in source repository if migrating.

### Common Signatures

1. Integration tests fail while crate lib tests pass: likely environment or fixture state isolation issue.
2. Workspace build passes but host runtime fails: likely endpoint/config mismatch.

## Parser and Validation Errors

### Symptom

Stored node rejected or strict parser fails.

### Actions

1. Confirm four-layer ordering is preserved.
2. Confirm content keys use field_name(.confidence) format.
3. Confirm content nesting depth does not exceed five.

### Common Signatures

1. Missing required layer errors indicate malformed STTP spine.
2. Invalid key format errors indicate content keys not using field_name(.confidence).
3. Nesting depth errors indicate recursive payload exceeded protocol constraints.

## Retrieval Behavior Mismatch

### Symptom

Unexpected recall ranking or fallback path.

### Actions

1. Log full scoring policy and scope.
2. Verify alpha and beta values are explicitly set.
3. Use explain workflow to inspect fallback triggers and stage counts.

### Common Signatures

1. Unexpected lexical_fallback path often indicates sparse embeddings or strict filter scope.
2. Score drift with unchanged input usually indicates policy defaults changed between versions.

## Transform/Migration Issues

### Symptom

Embedding backfill updates fewer nodes than expected.

### Actions

1. Run dry-run first and inspect selected counts.
2. Validate filter scope and has_embedding settings.
3. Validate provider capability routing and model configuration.

### Common Signatures

1. Selected count near zero in dry-run usually indicates filter constraints are too narrow.
2. Update counts diverge from selected counts when provider or storage writes are failing.

## Escalation Data To Capture

1. Exact request payload.
2. Retrieval path and fallback reason.
3. Validation and parser error text.
4. Relevant crate versions and commit sha.
