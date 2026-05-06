# Architecture

## Audience and Use

This page is for engineers deciding how to adopt Locus in production systems.

Use it to:

1. Choose where Locus should sit in your stack.
2. Understand component responsibilities before integration starts.
3. Avoid coupling host transports to internal implementation details.

## Purpose

Locus is the Rust implementation layer for STTP memory systems. STTP remains the protocol layer. Locus provides reusable components for parsing, validation, storage, retrieval, and composition with stable contract boundaries.

## Layering

1. Protocol layer: STTP grammar and four-layer node contract.
2. Implementation core: locus-core-rs.
3. SDK/application layer: locus-sdk.
4. Host transport layer: MCP, HTTP, gRPC hosts that consume Locus crates.

## Crate Responsibilities

### locus-core-rs

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

1. Deterministic-first behavior where possible.
2. Explicit policy controls over hidden defaults.
3. Transport-neutral workflows.
4. Additive evolution with compatibility preservation.
5. Observable retrieval paths with explainable outcomes.

## Data Contract Boundaries

1. STTP node text is validated against the protocol structure.
2. Core models carry parsed and computed fields.
3. SDK contracts define application-level behavior and policy.
4. Transport DTOs convert cleanly into SDK contracts.

## Dependency Direction

1. locus-sdk depends on locus-core-rs.
2. locus-core-rs has no dependency on locus-sdk.
3. Hosts should depend on locus-sdk for workflow orchestration and on locus-core-rs only when low-level access is required.

This dependency model keeps host contracts stable while internal implementations evolve.
