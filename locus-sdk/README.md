# locus-sdk

SDK-first STTP memory primitives and AI provider abstraction.

locus-sdk is the transport-agnostic core for STTP memory operations. It provides typed memory primitives, AI provider capability routing, and composition workflows that can be reused from MCP, HTTP, gRPC, CLI, or local applications.

## Why This Crate Exists

Historically, much of STTP memory behavior lived in transport layers. This crate moves core behavior into one SDK so the same semantics are shared everywhere.

Core goals:

1. Primitive-first API surface.
2. Transport-neutral contracts.
3. Deterministic policy behavior.
4. Composable workflows over stable primitives.
5. Single provider orchestration surface for embeddings and AVEC tasks.

## Current Scope

Implemented primitives and services:

1. memory_find
2. memory_recall
3. memory_aggregate
4. memory_transform
5. memory_explain
6. memory_schema

Implemented composition workflows:

1. recall_with_explain
2. daily_rollup
3. transform_then_recall_verify
4. capability_bundle
5. build_content_from_text (recursive deterministic node-from-text composite)

## Primitive Matrix

This matrix is intended as a fast contract reference for users, maintainers, and assistants.

| Primitive | Primary Input | Primary Output | Deterministic Guarantee |
| --- | --- | --- | --- |
| memory_find | scope + filter + sort + page | filtered nodes + cursor state | Non-ranked filtering and stable sort semantics |
| memory_recall | scope/filter/page + current_avec + optional query_text/query_embedding + scoring policy | ranked nodes + psi range + retrieval path | Policy-driven fallback behavior with explicit retrieval path |
| memory_aggregate | scope/filter + group_by + limits | grouped stats and totals | Stable grouping and bounded aggregation windows |
| memory_transform | selector + operation + dry_run + execution controls | mutation execution summary + failures | Explicit dry-run and bounded batch execution |
| memory_explain | recall request payload | stage counts + fallback reason + scoring profile | Explain trace derived from the same recall contract |
| memory_schema | none | supported fields, modes, and operations | Introspection is explicit and versioned |

## Composition Matrix

| Workflow | Uses | Best For |
| --- | --- | --- |
| recall_with_explain | memory_recall + memory_explain | ranked retrieval with transparent reasoning |
| daily_rollup | memory_aggregate grouped by day | timeline summaries and dashboards |
| transform_then_recall_verify | memory_transform + memory_recall | migration/backfill verification loops |
| capability_bundle | memory_schema | dynamic client and agent capability discovery |
| build_content_from_text | manual_compression + composition policy resolver | recursive deterministic content-layer construction from role-tagged text |

## Deterministic Compression and Composite Construction

This SDK supports deterministic text-to-content construction for STTP workflows without requiring model summarization for the core compression path.

Key building blocks:

1. Manual compression service with pluggable lexicon provider trait.
2. Request-level lexicon overrides (stopwords/fillers/negations add/remove).
3. Recursive composition workflow that builds spec-safe content payloads from role-tagged text input.

Core AVEC resolution chain in composite workflow:

1. item-level override
2. role-level override
3. global override
4. optional LLM fallback if enabled
5. explicit failure when unresolved and fallback disabled

Recursion guarantees:

1. Recursion depth is clamped to [1, 5].
2. Depth overflow fails fast.
3. Output content is structured for strict parser and validator compatibility.

Full guide:

1. [../docs/README.md](../docs/README.md)
2. [../docs/examples.md](../docs/examples.md)
3. [../docs/operations.md](../docs/operations.md)

## Crate Features

Default features:

1. genai-provider

Feature flags:

1. genai-provider: enables genai-based external provider adapter support.

## Architecture

Module layout:

1. domain
2. application
3. infrastructure
4. interface
5. prelude

Layer intent:

1. domain: contracts, enums, request/response models, policy types.
2. application: primitive services and composition orchestration.
3. infrastructure: provider adapters and registries.
4. interface: transport-friendly DTOs and conversion glue.
5. prelude: ergonomic re-exports for SDK consumers.

## Quick Start

Add dependency from workspace:

```toml
[dependencies]
locus-sdk = { path = "../locus-sdk" }
locus-core-rs = { path = "../locus-core-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Example: recall from an in-memory store

```rust
use std::sync::Arc;

use anyhow::Result;
use locus_core_rs::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{MemoryRecallRequest, MemoryRecallService};

#[tokio::main]
async fn main() -> Result<()> {
	let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
	let recall = MemoryRecallService::new(store);

	let result = recall.execute(&MemoryRecallRequest::default()).await?;
	println!("retrieved={}", result.retrieved);

	Ok(())
}
```

## Full End-to-End Example

Run a complete in-memory workflow that exercises the core primitives:

1. memory_find
2. memory_recall
3. memory_explain
4. memory_aggregate
5. memory_transform (dry-run then apply)
6. memory_schema
7. recall_with_explain composition

Command:

```bash
cargo run -p locus-sdk --example full_end_to_end
```

## Human and Assistant Collaboration Guide

This SDK is intentionally designed to be usable by both application developers and model-driven collaborators.

Recommended collaboration pattern:

1. Use DTOs at transport boundaries and domain models inside application logic.
2. Keep policy explicit in request payloads, especially fallback_policy and strictness.
3. Prefer composition services when you need traceable, multi-step memory workflows.
4. Treat memory_schema as the contract source for dynamic UIs and agent tool planning.

Assistant-safe request construction checklist:

1. Always set an explicit page.limit.
2. Provide scoring.alpha and scoring.beta when semantic retrieval is used.
3. Choose fallback_policy intentionally, do not rely on hidden defaults in transport wrappers.
4. Capture retrieval_path and explain stages when decisions must be auditable.

Determinism tips:

1. Use strict filters and narrow scope for reproducible retrieval windows.
2. Keep tier and date ranges explicit.
3. Preserve request payloads in logs when debugging ranking behavior.

## Model-First Playbook

This section is for assistants and collaborative agents that need reliable primitive selection and predictable payload construction.

### Primitive Selection Decision Tree

Use this order:

1. Need direct filtering without ranking: use memory_find.
2. Need ranked retrieval by AVEC and optional semantic signal: use memory_recall.
3. Need grouped statistics or rollups: use memory_aggregate.
4. Need bulk mutation or embedding backfill/reindex: use memory_transform.
5. Need retrieval reasoning trace: use memory_explain or recall_with_explain.
6. Need runtime capability discovery: use memory_schema or capability_bundle.
7. Need a multi-step verification flow: use composition workflows.

### Prompt Templates for Assistants

Template: deterministic recall

```text
Goal: Retrieve top memory nodes with deterministic policy behavior.
Constraints:
1. Explicit limit and scope.
2. Explicit alpha/beta.
3. Explicit fallback_policy.
4. Return retrieval_path and psi_range.
Action:
1. Build MemoryRecallRequest.
2. Execute MemoryRecallService.
3. If auditability required, execute MemoryExplainService with the same recall payload.
```

Template: migration verification

```text
Goal: Run embedding backfill and verify retrieval quality did not regress.
Action:
1. Execute transform_then_recall_verify.
2. Inspect transform.updated, transform.failed.
3. Inspect recall.retrieved and retrieval_path.
4. Record failures and fallback behavior.
```

Template: dynamic client introspection

```text
Goal: Build client-side controls from SDK capabilities.
Action:
1. Execute capability_bundle.
2. Populate UI controls from sort_fields, filter_fields, group_by_fields.
3. Populate policy selectors from fallback_policies and strictness_modes.
```

### Payload Examples

Example: MemoryRecallRequest payload (json-style)

```json
{
	"scope": {
		"sessionIds": ["session-abc"],
		"tiers": ["raw"],
		"fromUtc": "2026-05-01T00:00:00Z",
		"toUtc": "2026-05-03T00:00:00Z"
	},
	"filter": {
		"hasEmbedding": true,
		"textContains": "parser"
	},
	"page": {
		"limit": 25,
		"cursor": null
	},
	"scoring": {
		"resonanceWeight": 1.0,
		"semanticWeight": 0.5,
		"lexicalWeight": 0.1,
		"alpha": 0.7,
		"beta": 0.3,
		"fallbackPolicy": "on_empty",
		"strictness": "balanced"
	},
	"currentAvec": {
		"stability": 0.82,
		"friction": 0.28,
		"logic": 0.91,
		"autonomy": 0.76,
		"psi": 2.77
	},
	"queryText": "typed ir parser hardening"
}
```

Example: MemoryTransformThenRecallRequest payload (json-style)

```json
{
	"transform": {
		"scope": {
			"sessionIds": ["session-abc"]
		},
		"filter": {
			"hasEmbedding": false
		},
		"operation": "embed_backfill",
		"dryRun": false,
		"batchSize": 100,
		"maxNodes": 5000,
		"providerId": "genai",
		"model": "text-embedding-3-large"
	},
	"recall": {
		"scope": {
			"sessionIds": ["session-abc"]
		},
		"page": {
			"limit": 20,
			"cursor": null
		},
		"scoring": {
			"alpha": 0.7,
			"beta": 0.3,
			"fallbackPolicy": "on_empty",
			"strictness": "balanced",
			"resonanceWeight": 1.0,
			"semanticWeight": 0.5,
			"lexicalWeight": 0.1
		},
		"queryText": "typed ir spec"
	}
}
```

### Assistant Reliability Checklist

1. Never omit page.limit.
2. Keep scope bounded whenever possible.
3. Set fallback_policy explicitly.
4. Log retrieval_path for every recall execution.
5. Use explain for user-facing rationale.
6. Use schema to avoid hardcoding enum assumptions.

## Composition Example

Use the composition service for higher-level workflows:

1. recall with explain trace.
2. daily rollups.
3. transform then verify via recall.

See runnable examples:

1. [examples/provider_registry_setup.rs](examples/provider_registry_setup.rs)
2. [examples/memory_composition.rs](examples/memory_composition.rs)
3. [examples/recursive_composite_pipeline.rs](examples/recursive_composite_pipeline.rs)

Primitive-oriented implementation references:

1. [src/application/memory_find.rs](src/application/memory_find.rs)
2. [src/application/memory_recall.rs](src/application/memory_recall.rs)
3. [src/application/memory_aggregate.rs](src/application/memory_aggregate.rs)
4. [src/application/memory_transform.rs](src/application/memory_transform.rs)
5. [src/application/memory_explain.rs](src/application/memory_explain.rs)
6. [src/application/memory_schema.rs](src/application/memory_schema.rs)
7. [src/application/memory_composition.rs](src/application/memory_composition.rs)
8. [src/application/manual_compression.rs](src/application/manual_compression.rs)

Synthetic fixture generator:

1. [examples/generate_faker_fixture.rs](examples/generate_faker_fixture.rs)

## AI Provider Model

Provider orchestration is capability-driven.

Capabilities:

1. SemanticEmbedding
2. AvecEmbedding
3. AvecScoring

Tasks and provider selection are expressed through typed contracts:

1. AiTask
2. ProviderPolicy
3. EmbedRequest
4. ScoreAvecRequest
5. AiProvider and AiProviderRegistry

## Public Surface

Import from prelude for most use cases:

```rust
use locus_sdk::prelude::*;
```

prelude exports include:

1. primitive request models.
2. primitive services.
3. composition services and request/result types.
4. DTOs for transport integration.
5. provider contracts and registry adapters.

## Primitive Recipes

Use these as tiny patterns when composing workflows.

### Recipe: Find Nodes

```rust
use locus_sdk::prelude::{MemoryFindRequest, MemoryFindService};

let service = MemoryFindService::new(store);
let result = service.execute(&MemoryFindRequest::default()).await?;
```

### Recipe: Recall with Explicit Fallback Policy

```rust
use locus_sdk::prelude::{FallbackPolicy, MemoryRecallRequest, MemoryRecallService, MemoryScoring};

let service = MemoryRecallService::new(store);
let request = MemoryRecallRequest {
	scoring: MemoryScoring {
		fallback_policy: FallbackPolicy::OnEmpty,
		..Default::default()
	},
	..Default::default()
};
let result = service.execute(&request).await?;
```

### Recipe: Transform then Verify

```rust
use locus_sdk::prelude::{
	MemoryCompositionService, MemoryRecallRequest, MemoryTransformOperation,
	MemoryTransformRequest, MemoryTransformThenRecallRequest,
};

let composition = MemoryCompositionService::new(store);
let verify = composition
	.transform_then_recall_verify(
		providers,
		&MemoryTransformThenRecallRequest {
			transform: MemoryTransformRequest {
				operation: MemoryTransformOperation::EmbedBackfill,
				..Default::default()
			},
			recall: MemoryRecallRequest::default(),
		},
	)
	.await?;
```

### Recipe: Build Recursive Content from Text

```rust
use locus_core_rs::domain::models::AvecState;
use locus_sdk::prelude::{
	CompositeInputItem, CompositeNodeFromTextOptions, CompositeNodeFromTextRequest,
	CompositeRole, CompositeRoleAvecOverrides, MemoryCompositionService,
};

let composition = MemoryCompositionService::new(store);

let request = CompositeNodeFromTextRequest {
	items: vec![CompositeInputItem {
		role: CompositeRole::Conversation,
		text: "user asks for deterministic recall and model explains fallback".to_string(),
		avec_override: None,
		context: vec![CompositeInputItem {
			role: CompositeRole::Document,
			text: "internal notes: lexical fallback on empty".to_string(),
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
		global_avec: None,
		allow_llm_avec_fallback: false,
		max_recursion_depth: 5,
	},
};

let result = composition.build_content_from_text(&request)?;
println!("resolved_avec_count={}", result.resolved_avec_count);
println!("content={}", result.content);
```

## Behavior Notes

1. Memory recall supports policy-controlled lexical fallback behavior.
2. Explain returns stage-level visibility into retrieval pipeline behavior.
3. Schema exposes discoverable primitive capabilities for dynamic clients.
4. Transform supports dry-run and batch controls for safer bulk operations.

## Development

From crate root:

```bash
cargo test
cargo check --examples
cargo run --example recursive_composite_pipeline
```

Generate synthetic JSONL fixtures:

```bash
cargo run --example generate_faker_fixture -- \
	--seed 20260503 \
	--sessions 8 \
	--min-nodes 12 \
	--max-nodes 20 \
	--filler-ratio 0.24 \
	--topic-drift 0.30 \
	--span-days 45 \
	--output ../docs/example_data/pipeline/sttp_faker_fixture_v1.jsonl
```

## Integration Status

As of 2026-05-03:

1. SDK phases 0 through 4 are complete.
2. Composition layer and DTO surface are in place, including recursive node-from-text composite contracts.
3. Deterministic manual compression is implemented with trait-based lexicon customization.
4. Transport migration has started in MCP and Gateway using SDK-backed retrieval paths.

Architecture and rollout plan:

1. [../docs/architecture.md](../docs/architecture.md)
2. [../docs/integration.md](../docs/integration.md)

## Transport Migration Cookbook

This section is for MCP and Gateway maintainers adopting SDK primitives behind existing endpoints.

Migration strategy:

1. Keep external endpoint shapes stable.
2. Map transport request models into SDK domain requests.
3. Execute SDK services in handlers.
4. Map SDK results back to existing response models.
5. Preserve compatibility wrappers until at least one release cycle after migration.

Suggested order:

1. get_context -> memory_recall
2. list_nodes -> memory_find
3. rollup endpoints -> memory_aggregate composition recipes
4. embedding migration endpoints -> memory_transform
5. explain and schema endpoints -> memory_explain and memory_schema

Mapping checklist per endpoint:

1. scope: tenant/session/tier/time fields map to MemoryScope.
2. pagination: map limit/cursor to MemoryPage.
3. scoring: map alpha/beta/fallback policy to MemoryScoring.
4. filters: map typed ranges to MemoryFilter.
5. response: preserve previous wire fields while sourcing values from SDK results.

Acceptance checklist:

1. Existing endpoint tests still pass.
2. Retrieval path and fallback behavior are explicit in logs.
3. New integration tests validate parity between old and SDK-backed behavior.
4. Feature flags or rollout toggles are available for controlled cutover.

## Non-Goals for This Crate

1. Storage backend implementation details.
2. Transport endpoint definitions.
3. STTP typed IR grammar or parser conformance logic.

For typed IR language and grammar contract work, refer to:

1. [../docs/sttp_typed_ir_language_spec.md](../docs/sttp_typed_ir_language_spec.md)
2. [../docs/troubleshooting.md](../docs/troubleshooting.md)
