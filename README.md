# locus

Locus is the Rust memory implementation layer for the STTP protocol.

This repository is split from the prior monorepo so memory implementation concerns can evolve independently while STTP remains the protocol layer.

## Workspace Crates

1. `locus-core`: core storage, parsing, validation, and sync-ready primitives for STTP nodes.
2. `locus-sdk`: SDK-first memory primitives, composition workflows, and provider routing.

## Quick Start

```bash
cargo test
cargo check --examples
```

## Documentation

1. `docs/README.md`
2. `docs/architecture.md`
3. `docs/deployment.md`
4. `docs/operations.md`
5. `docs/integration.md`
6. `docs/examples.md`
7. `docs/troubleshooting.md`
8. `docs/versioning.md`
9. `docs/security.md`
10. `docs/sttp_typed_ir_language_spec.md`
