# Examples

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

## Production Usage Guidance

1. Treat examples as reference patterns, not deployment templates.
2. Externalize credentials and environment-specific endpoints.
3. Keep explicit limits and fallback policies in runtime requests.
