# Examples

## Audience and Use

This page is for developers using Locus patterns in their own systems.

Use examples to:

1. Start from known-good integration patterns.
2. Validate behavior quickly in local environments.
3. Translate reference workflows into production service code.

## Available Example Binaries

1. ../locus-sdk/examples/provider_registry_setup.rs
2. ../locus-sdk/examples/memory_composition.rs
3. ../locus-sdk/examples/recursive_composite_pipeline.rs
4. ../locus-sdk/examples/generate_faker_fixture.rs

## Run Examples

From repository root:

```bash
cargo run -p locus-sdk --example provider_registry_setup
cargo run -p locus-sdk --example memory_composition
cargo run -p locus-sdk --example recursive_composite_pipeline
cargo run -p locus-sdk --example generate_faker_fixture -- --help
```

## Example Coverage

1. Provider capability registry wiring.
2. Composition workflows and multi-step orchestration.
3. Recursive deterministic content construction plus strict parser and validator checks.
4. Deterministic fixture generation for load and quality testing.

For domain-specific multi-agent setup patterns, see [Agent Blueprints by Domain](agent-blueprints.md).

For practical GenAI plus STTP memory orchestration recipes, see [Mini Orchestration Cookbooks](orchestration-cookbook.md).

If you want a fast path chooser by outcome and build style, start with [Cookbook Overview](cookbooks-overview.md).

## Example Decision Guide

### provider_registry_setup

Use when:

1. You are wiring provider capability routing for the first time.
2. You need to validate task-to-provider matching rules.

Expected outcome:

1. Registry contains expected providers and capability visibility.
2. Unsupported provider and task combinations fail with explicit errors.

Common failure checks:

1. Provider IDs differ between config and runtime registration.
2. Required capabilities are missing from selected providers.

Next integration step:

1. Wire registry initialization into your service startup path and fail fast on missing capabilities.

### memory_composition

Use when:

1. You want a reference for multi-step memory workflows.
2. You need recall and explain behavior in one composed flow.

Expected outcome:

1. Composed workflow returns stable result shapes.
2. Explain path aligns with recall outcome.

Common failure checks:

1. Request policy settings are implicit instead of explicit.
2. Scope filters are omitted, causing noisy result sets.

Next integration step:

1. Promote composed request defaults into a shared app-level policy module.

### recursive_composite_pipeline

Use when:

1. You are building structured content from recursive composition.
2. You need deterministic recursion-depth behavior.

Expected outcome:

1. Recursive output respects depth and schema constraints.
2. Content remains parser and validator compatible.

Common failure checks:

1. Recursion depth exceeds configured limits.
2. Output shape drifts from expected typed content structure.

Next integration step:

1. Add schema-level response assertions in CI to prevent output drift before deployment.

### generate_faker_fixture

Use when:

1. You need reproducible fixture data for tests and benchmarking.
2. You want fast local workload simulation.

Expected outcome:

1. Fixture generation is deterministic for the same parameters.
2. Generated payloads are valid for ingest and retrieval tests.

Common failure checks:

1. Parameter ranges produce unrealistic payload distributions.
2. Fixture output is not validated before being used in regression tests.

Next integration step:

1. Add fixture generation to CI pre-test setup with deterministic seed controls.

## Production Usage Guidance

1. Treat examples as reference patterns, not deployment templates.
2. Externalize credentials and environment-specific endpoints.
3. Keep explicit limits and fallback policies in runtime requests.
4. Add environment-specific observability and failure handling before production rollout.

## Promotion Checklist

Before promoting an example pattern into production code:

1. Add scope controls and policy defaults explicitly.
2. Add structured error handling and retry boundaries.
3. Add telemetry for latency, fallback behavior, and failures.
4. Add regression tests for request and response shape stability.
