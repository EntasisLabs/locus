# Security and Data Handling

## Security Posture

Locus is a memory implementation layer and may process sensitive conversational state. Treat all stored node content as potentially sensitive.

## Data Classification Guidance

1. Treat raw node text and content layer fields as confidential by default.
2. Treat session identifiers and tenant identifiers as sensitive metadata.
3. Treat provider credentials and endpoint secrets as secrets at all times.

## Secret Management

1. Never commit credentials, tokens, or endpoint secrets to source control.
2. Use environment variables or secret managers for runtime configuration.
3. Rotate credentials on incident response or role changes.

## PII and Sensitive Content

1. Avoid storing direct PII when not required for product behavior.
2. Apply redaction/minimization before persistence where possible.
3. Ensure logs do not include raw secrets or unredacted sensitive payloads.

## Transport and Storage Controls

1. Use TLS-enabled transports for remote database and provider calls.
2. Restrict database access using least privilege credentials.
3. Scope operations by tenant/session to avoid cross-tenant data leakage.

## Operational Security Checklist

1. Validate dependency updates and lockfile changes during review.
2. Run parser/validator tests to avoid unsafe ingestion regressions.
3. Audit error paths for accidental raw payload leakage.
4. Keep incident response runbooks updated with containment steps.

## Incident Handling Baseline

1. Contain by rotating keys and restricting affected endpoints.
2. Identify scope using session/tenant trace data.
3. Patch and verify with deterministic replay payloads.
4. Document remediation and prevention actions in release notes.
