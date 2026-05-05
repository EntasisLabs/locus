# Locus Documentation

This documentation set is focused on production usage of the Locus Rust memory implementation for the STTP protocol.

## Start Here

1. architecture.md
2. deployment.md
3. operations.md
4. integration.md
5. examples.md
6. troubleshooting.md
7. versioning.md
8. security.md
9. sttp_typed_ir_language_spec.md

## Audience Map

1. Platform teams: architecture.md, deployment.md, operations.md
2. Backend integrators: integration.md, examples.md
3. On-call and SRE: operations.md, troubleshooting.md, security.md
4. Release managers: versioning.md
5. Protocol authors: sttp_typed_ir_language_spec.md

## Repository Components

1. ../locus-core
2. ../locus-sdk

## Generated Technical Docs

Locus technical docs can be generated as a full site combining guides and API docs.

From repo root:

```bash
./docs/build-technical-docs.sh
```

Tooling prerequisites:

1. `cargo install mdbook`
2. `cargo install mdbook-mermaid`

Outputs:

1. technical/book/index.html (mdBook chapters)
2. technical/api/index.html (rustdoc API reference)
3. technical/index.html (combined entrypoint)
