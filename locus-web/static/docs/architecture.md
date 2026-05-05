# Architecture

## Purpose

Locus is the Rust implementation layer for STTP memory systems. STTP remains the protocol layer. Locus provides reusable, production-oriented components for parsing, validation, storage, retrieval, and composition.

## Layering

1. Protocol layer: STTP grammar and four-layer node contract.
2. Implementation core: locus-core.
3. SDK/application layer: locus-sdk.
4. Host transport layer: MCP, HTTP, gRPC hosts that consume Locus crates.

## Crate Responsibilities

### locus-core

1. Domain models and contracts.
2. STTP parsing and validation.
3. Storage adapters (in-memory and SurrealDB).
4. Core services for calibration, store/query, rollups, and sync-ready primitives.

### locus-sdk

1. Primitive memory workflows: find, recall, aggregate, transform, explain, schema.
2. Composition workflows for multi-step operations.
3. Deterministic manual compression and recursive content construction.
4. Transport DTOs and typed request/response boundaries.

## Design Principles

1. Deterministic-first where possible.
2. Explicit policy controls over hidden defaults.
3. Transport neutrality.
4. Backward-compatible, additive evolution.
5. Observability and explainability in retrieval paths.

## Data Contract Boundaries

1. STTP node text is validated against the protocol structure.
2. Core models carry parsed and computed fields.
3. SDK contracts define application-level behavior and policy.
4. Transport DTOs convert cleanly into SDK contracts.

## Dependency Direction

1. locus-sdk depends on locus-core.
2. locus-core has no dependency on locus-sdk.
3. Hosts should depend on locus-sdk for workflow orchestration and on locus-core only when low-level access is required.
