# locus-core-rs

Core Rust library for STTP domain modeling, validation, parsing, storage, and sync-ready coordination primitives.

This crate is designed for Rust apps, MCP servers, gateways, and services that need to store and retrieve STTP nodes with AVEC-aware semantics while staying open to future cloud/local sync scenarios.

## What It Is Good At

`locus-core-rs` is meant to do the reusable, low-level work well:

- parse and validate STTP nodes
- persist and retrieve nodes consistently
- support AVEC-aware retrieval and rollups
- provide sync-ready mechanics without forcing application-specific sync rules

It is not a full product framework. It does not decide which side is authoritative, how conflicts should be resolved, or when synchronization should run.

## What You Get

- Domain contracts and models for STTP and AVEC.
- Application services:
  - calibration,
  - context retrieval,
  - context storage,
  - mood catalog and blend preview,
  - monthly rollup generation,
    - batch scope rekey,
    - sync coordination with pluggable source and policy hooks.
- Storage implementations:
  - in-memory store,
  - SurrealDB-backed store.
- Sync primitives:
    - deterministic sync keys for STTP nodes,
    - idempotent node upserts,
    - incremental change queries with cursors,
    - connector checkpoints for cloud/local sync state,
    - typed connector metadata for provenance,
    - a narrow coordinator surface for paging changes and advancing checkpoints.
- Parser and validator primitives for raw STTP node text.

## Backward Compatibility

The sync-related additions are designed to be additive.

- Existing nodes that do not have `sync_key`, `updated_at`, or connector metadata still load correctly.
- Existing callers can continue using the simple store/query APIs.
- If you never implement cloud/local sync, the crate still works as a normal STTP storage and retrieval library.

In other words: the crate is sync-ready, not sync-mandatory.

## Installation

```toml
[dependencies]
locus-core-rs = "0.1.4"
```

## Quick Start

```rust,no_run
use std::sync::Arc;

use locus_core_rs::{
    CalibrationService, ContextQueryService, InMemoryNodeStore, MoodCatalogService,
    NodeStoreInitializer,
};

# fn main() -> anyhow::Result<()> {
let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?;

runtime.block_on(async {
    let store = Arc::new(InMemoryNodeStore::new());
    let initializer: Arc<dyn NodeStoreInitializer> = store.clone();
    initializer.initialize_async().await?;

    let calibration = CalibrationService::new(store.clone());
    let context = ContextQueryService::new(store.clone());
    let moods = MoodCatalogService::new();

    let calibration_result = calibration
        .calibrate_async("session-1", 0.9, 0.2, 0.9, 0.85, "manual")
        .await?;

    let retrieved = context
        .get_context_async("session-1", 0.9, 0.2, 0.9, 0.85, 5)
        .await;

    let catalog = moods.get(Some("focused"), 1.0, Some(0.8), Some(0.2), Some(0.8), Some(0.8));

    println!("delta={} retrieved={} presets={}", calibration_result.delta, retrieved.retrieved, catalog.presets.len());

    Ok::<(), anyhow::Error>(())
})?;
# Ok(())
# }
```

## Public API Surface

- Services: `CalibrationService`, `ContextQueryService`, `StoreContextService`, `MoodCatalogService`, `MonthlyRollupService`, `RekeyScopeService`, `SyncCoordinatorService`.
- Validation: `TreeSitterValidator`.
- Storage: `InMemoryNodeStore`, `SurrealDbNodeStore`, `SurrealDbRuntimeOptions`, `SurrealDbSettings`.
- Contracts: `NodeStore`, `NodeStoreInitializer`, `NodeValidator`, `SyncChangeSource`, `SyncCoordinatorPolicy`.
- Core models: `SttpNode`, `AvecState`, `NodeQuery`, `MonthlyRollupRequest`, `BatchRekeyResult`, `NodeUpsertResult`, `SyncCursor`, `SyncCheckpoint`, `SyncPullRequest`, `SyncPullResult`, `ConnectorMetadata`.

## Sync Model In Plain English

The sync surface is intentionally narrow.

- The crate can identify nodes deterministically.
- It can tell you what changed since a cursor.
- It can remember where a connector last stopped.
- It can coordinate the mechanics of pulling changes and storing them.

What it does not do is own business logic.

- It does not decide whether cloud or local is the source of truth.
- It does not decide conflict resolution.
- It does not schedule sync.
- It does not know anything about your product rules.

Those decisions belong in the host application.

## Build And Test

```bash
cargo check --manifest-path locus-core-rs/Cargo.toml
cargo test --manifest-path locus-core-rs/Cargo.toml
```

## Build Package Artifact

Create a local `.crate` artifact for verification:

```bash
cargo package --manifest-path locus-core-rs/Cargo.toml --allow-dirty
```

Inspect package contents:

```bash
cargo package --manifest-path locus-core-rs/Cargo.toml --allow-dirty --list
```

The artifact is written under `locus-core-rs/target/package`.

## First Publish To crates.io (Step-By-Step)

If this is your first crates.io release, use this sequence.

### 1. Verify Name Availability

```bash
cargo search locus-core-rs --limit 5
```

If the exact crate name is already taken, update `[package].name` in `Cargo.toml`.

### 2. Create crates.io Token

1. Sign in at crates.io.
2. Go to Account Settings -> API Tokens.
3. Create a token with publish scope.

Login locally:

```bash
cargo login <YOUR_CRATES_IO_TOKEN>
```

Alternative for CI/non-interactive use:

```bash
export CARGO_REGISTRY_TOKEN=<YOUR_CRATES_IO_TOKEN>
```

### 3. Release Readiness Checks

From repository root:

```bash
cargo check --manifest-path locus-core-rs/Cargo.toml
cargo test --manifest-path locus-core-rs/Cargo.toml
```

Check exact package contents:

```bash
cargo package --manifest-path locus-core-rs/Cargo.toml --list
```

### 4. Dry-Run Publish (Required)

```bash
cargo publish --manifest-path locus-core-rs/Cargo.toml --dry-run
```

### 5. Publish

```bash
cargo publish --manifest-path locus-core-rs/Cargo.toml
```

### 6. Post-Publish Verification

```bash
cargo info locus-core-rs
```

Then verify docs build on docs.rs (can take a few minutes).

## Optional: One-Command Release Preflight

Use the helper script:

```bash
./locus-core-rs/publish-crates.sh
```

To run publish after preflight:

```bash
./locus-core-rs/publish-crates.sh --publish
```
