# STTP Document Builder

Status: Implemented (initial)
Last Updated: 2026-08-25

## Vision

```rust
SttpDocumentBuilder::new(metadata)
    .merge(core_slice)?
    .merge(mode_slice)?
    .merge(turn_slice)?
    .build()?
    .render_canonical()
```

## Locked merge contract

1. **Metadata owns outer layers.** Provenance (`⊕`) and envelope (`⦿`) are seeded only from `SttpDocumentMetadata`. Slices cannot patch them.
2. **Merge is shallow and content-only.** Each `SttpContentSlice` may contribute top-level `◈` content keys only.
3. **Nested values are opaque.** Objects/arrays under a top-level key are never deep-merged.
4. **Duplicate field names fail.** Collision is by field name (`core`), not full key (`core(.98)`). A later slice that reuses a name returns `DuplicateContentField`.
5. **Metrics finalize at build.** `rho` / `kappa` / `compression_avec` / `psi` are taken from metadata overrides or derived defaults during `build()`.
6. **Canonical render targets StrictTypedIr.** `render_canonical()` emits the four-layer spine and must round-trip through `TreeSitterValidator` + `try_parse_strict_typed_ir`.

## Types

Defined in `locus-core-rs/src/parsing/document_builder.rs`:

| Type | Role |
|---|---|
| `SttpDocumentMetadata` | Session, tier, AVEC, tags/links, optional metrics overrides |
| `SttpContentSlice` | Top-level content contribution (`field` / `from_confidence_map`) |
| `SttpDocumentBuilder` | Fluent `new` → `merge` → `build` |
| `SttpDocument` | Built document with `render_canonical()` |
| `SttpDocumentBuildError` | Typed construction failures |

## Example

```rust
use locus_core_rs::{
    AvecState, SttpContentSlice, SttpDocumentBuilder, SttpDocumentMetadata,
};
use serde_json::json;

let metadata = SttpDocumentMetadata::new("session")
    .with_context_summary("builder demo")
    .with_avec(AvecState::analytical(), AvecState::analytical());

let core = SttpContentSlice::new()
    .field("core", 0.98, json!({ "focus(.99)": "merge policy" }))?;
let mode = SttpContentSlice::new()
    .field("mode", 0.97, json!({ "profile(.99)": "strict" }))?;

let wire = SttpDocumentBuilder::new(metadata)
    .merge(core)?
    .merge(mode)?
    .build()?
    .render_canonical();
```

## Non-goals (v1)

1. Deep merge inside nested content objects
2. Slice contributions to provenance / envelope / metrics
3. Last-write-wins overwrite of duplicate top-level names
