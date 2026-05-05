# STTP SDK Recursive Node-From-Text Composite Guide

Date: 2026-05-03
Status: Implemented (initial slice)

## 1. Purpose

This guide documents the recursive deterministic composition workflow in locus-sdk.

The workflow lets callers build a spec-safe STTP content-layer payload from plain text entries without requiring model summarization for the core compression path.

## 2. What It Solves

1. Build content payloads from ordered role-tagged text input.
2. Support recursive context trees while staying validator-safe.
3. Resolve AVEC using explicit deterministic policy order.
4. Keep an optional LLM path only for unresolved AVEC, not core compression.

## 3. Core Types

Defined in [locus-sdk/src/application/memory_composition.rs](locus-sdk/src/application/memory_composition.rs):

1. CompositeRole
2. CompositeInputItem
3. CompositeRoleAvecOverrides
4. CompositeNodeFromTextOptions
5. CompositeNodeFromTextRequest
6. CompositeNodeFromTextResult

DTO equivalents are defined in [locus-sdk/src/interface/dto.rs](locus-sdk/src/interface/dto.rs):

1. CompositeRoleDto
2. CompositeInputItemDto
3. CompositeRoleAvecOverridesDto
4. CompositeNodeFromTextOptionsDto
5. CompositeNodeFromTextRequestDto
6. CompositeNodeFromTextResponseDto

## 4. Deterministic AVEC Resolution

For each input item, AVEC resolution order is:

1. item-level override (CompositeInputItem.avec_override)
2. role-level override (CompositeNodeFromTextOptions.role_avec)
3. global override (CompositeNodeFromTextOptions.global_avec)
4. unresolved

If unresolved item count is greater than zero:

1. if allow_llm_avec_fallback is false, the workflow fails with a typed error,
2. if allow_llm_avec_fallback is true, requires_llm_avec is true in the result.

## 5. Recursion and Validation Constraints

1. Requested depth is clamped to [1, 5].
2. Build fails if item context exceeds max_recursion_depth.
3. The content object uses confidence-scored keys to remain strict-parser compatible.
4. This aligns with validator nesting limits in core validator logic.

Reference: [locus-core/src/application/validation/tree_sitter_validator.rs](locus-core/src/application/validation/tree_sitter_validator.rs)

## 6. Compression Strategy

Each input/context item is compressed with ManualCompressionService to derive:

1. anchor_topic
2. key_points

This keeps the composite deterministic and model-optional for compression.

Reference: [locus-sdk/src/application/manual_compression.rs](locus-sdk/src/application/manual_compression.rs)

## 7. Quick Example (Domain Models)

```rust
use std::sync::Arc;

use anyhow::Result;
use locus_core::{InMemoryNodeStore, NodeStore};
use locus_core::domain::models::AvecState;
use locus_sdk::prelude::{
    CompositeInputItem, CompositeNodeFromTextOptions, CompositeNodeFromTextRequest,
    CompositeRole, CompositeRoleAvecOverrides, MemoryCompositionService,
};

fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    let composition = MemoryCompositionService::new(store);

    let req = CompositeNodeFromTextRequest {
        items: vec![CompositeInputItem {
            role: CompositeRole::Conversation,
            text: "user asks about fallback policy and model explains ranked retrieval".to_string(),
            avec_override: None,
            context: vec![CompositeInputItem {
                role: CompositeRole::Document,
                text: "design notes mention deterministic lexical fallback".to_string(),
                avec_override: None,
                context: Vec::new(),
            }],
        }],
        options: CompositeNodeFromTextOptions {
            role_avec: CompositeRoleAvecOverrides {
                conversation: Some(AvecState {
                    stability: 0.82,
                    friction: 0.20,
                    logic: 0.86,
                    autonomy: 0.74,
                }),
                ..Default::default()
            },
            global_avec: Some(AvecState {
                stability: 0.75,
                friction: 0.25,
                logic: 0.80,
                autonomy: 0.70,
            }),
            allow_llm_avec_fallback: false,
            max_recursion_depth: 5,
        },
    };

    let result = composition.build_content_from_text(&req)?;

    println!("resolved_avec_count={}", result.resolved_avec_count);
    println!("requires_llm_avec={}", result.requires_llm_avec);
    println!("content={}", result.content);

    Ok(())
}
```

## 8. Quick Example (DTO Boundary)

```rust
use locus_sdk::prelude::{
    CompositeNodeFromTextRequestDto, CompositeNodeFromTextResponseDto,
};

fn convert_boundary(req_dto: CompositeNodeFromTextRequestDto) -> CompositeNodeFromTextResponseDto {
    let domain_req = req_dto.into();
    let domain_result = service.build_content_from_text(&domain_req).unwrap();
    domain_result.into()
}
```

## 9. Test Coverage

Current coverage includes:

1. role and global AVEC resolution chain
2. unresolved AVEC failure behavior
3. recursion depth enforcement
4. strict parser and validator conformance for generated content payload

Reference tests: [locus-sdk/src/application/memory_composition.rs](locus-sdk/src/application/memory_composition.rs)

## 10. Integration Notes

1. This workflow currently returns content-layer payload plus AVEC resolution stats.
2. It does not replace typed IR grammar; it composes spec-safe content for insertion into full STTP node workflows.
3. Recommended next step is a full store-ready recipe that assembles the complete four-layer node from this content payload.
