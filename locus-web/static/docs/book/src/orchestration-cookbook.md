# Mini Orchestration Cookbooks

## Why This Exists

This chapter is for builders who want working patterns, not theory.

Use these cookbooks when you want:

1. GenAI to handle model-facing intelligence.
2. Locus to handle memory, recall policy, explainability, and safe transforms.
3. A repeatable orchestration shape you can adapt by domain.

For a fast picker by outcome, go to [Cookbook Overview](cookbooks-overview.md).

## Core Pattern

Every cookbook in this chapter follows the same loop:

1. Ingest events into STTP memory.
2. Recall context with explicit scoring and fallback policy.
3. Explain the retrieval path before high-impact actions.
4. Optionally run transform in dry-run before mutation.
5. Pass grounded context into a prompt template for model response.

## GenAI Hookup: Provider and Routing

Use GenAI as the model interface and keep memory orchestration in Locus.

```rust
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use locus_sdk::prelude::{
    AiRoutingConfig, InMemoryAiProviderRegistry, ProviderModelProfile,
};
#[cfg(feature = "genai-provider")]
use locus_sdk::prelude::GenaiProviderAdapter;

fn build_registry_and_routing() -> Result<(Arc<InMemoryAiProviderRegistry>, AiRoutingConfig)> {
    let mut registry = InMemoryAiProviderRegistry::new();

    #[cfg(feature = "genai-provider")]
    registry.register(GenaiProviderAdapter::new(
        "genai",
        Some("text-embedding-3-small".to_string()),
    ));

    let mut providers = HashMap::new();
    providers.insert(
        "genai".to_string(),
        ProviderModelProfile {
            semantic_model: Some("text-embedding-3-small".to_string()),
            avec_embedding_model: Some("text-embedding-3-small".to_string()),
            avec_scoring_model: Some("gpt-4o-mini".to_string()),
        },
    );

    let routing = AiRoutingConfig {
        default_provider_id: Some("genai".to_string()),
        providers,
    };

    Ok((Arc::new(registry), routing))
}
```

Integration note:

1. Configure credentials for your selected GenAI provider in environment variables.
2. Keep provider IDs stable across environments for predictable routing behavior.

## Composition Primer

The `MemoryCompositionService` gives you high-level orchestration:

1. `recall_with_explain` for grounded context with retrieval diagnostics.
2. `daily_rollup` for memory compression over time windows.
3. `transform_then_recall_verify` for safe write-preview and immediate validation.
4. `build_content_from_text` for deterministic content construction.

## Cookbook 1: Support Copilot (Fast Resolution)

Intent:

1. Pull the smallest high-confidence context window.
2. Generate a customer-safe next action grounded in prior resolutions.

```rust
use anyhow::Result;
use std::sync::Arc;
use locus_core::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    FallbackPolicy, MemoryCompositionService, MemoryRecallRequest, MemoryScoring,
};

#[tokio::main]
async fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    let composition = MemoryCompositionService::new(store);

    let result = composition
        .recall_with_explain(&MemoryRecallRequest {
            query_text: Some("billing mismatch after plan upgrade".to_string()),
            scoring: MemoryScoring {
                alpha: 0.70,
                beta: 0.30,
                fallback_policy: FallbackPolicy::OnEmpty,
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;

    println!(
        "retrieved={}, path={:?}, stages={}",
        result.recall.retrieved,
        result.explain.retrieval_path,
        result.explain.stages.len()
    );

    Ok(())
}
```

Prompt template:

```text
system:
You are a support resolution assistant.
Use only provided context. If evidence is weak, say so explicitly.

user:
Issue: {{live_issue_summary}}
Top memory context:
{{ranked_context_snippets}}

Return:
1) likely root cause
2) next best action
3) confidence (0-1)
4) what evidence is still missing
```

## Cookbook 2: Incident Triage Captain (Explain Before Escalate)

Intent:

1. Keep response speed high.
2. Require explain trace before severity escalation.

```rust
use anyhow::Result;
use std::sync::Arc;
use locus_core::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    FallbackPolicy, MemoryCompositionService, MemoryRecallRequest, MemoryScoring,
    StrictnessMode,
};

#[tokio::main]
async fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    let composition = MemoryCompositionService::new(store);

    let triage = composition
        .recall_with_explain(&MemoryRecallRequest {
            query_text: Some("unusual outbound auth failures".to_string()),
            scoring: MemoryScoring {
                alpha: 0.75,
                beta: 0.25,
                fallback_policy: FallbackPolicy::Always,
                strictness: StrictnessMode::Precision,
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;

    println!("path={:?}", triage.explain.retrieval_path);
    println!("fallback_triggered={}", triage.explain.fallback_triggered);
    Ok(())
}
```

Prompt template:

```text
system:
You are an incident triage assistant. Never recommend irreversible action without evidence.

user:
Signal summary: {{signal_summary}}
Explain trace:
{{explain_stages}}
Top evidence:
{{context_snippets}}

Return:
1) severity recommendation
2) containment candidates
3) risks of false positive
4) evidence citations
```

## Cookbook 3: Release Reliability Coach (Dry-Run First)

Intent:

1. Preview transform impact before writes.
2. Validate retrieval quality immediately after transform simulation.

```rust
use anyhow::Result;
use std::sync::Arc;
use locus_core::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    InMemoryAiProviderRegistry, MemoryCompositionService, MemoryRecallRequest,
    MemoryTransformOperation, MemoryTransformRequest, MemoryTransformThenRecallRequest,
};

#[tokio::main]
async fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    let composition = MemoryCompositionService::new(store);
    let providers = Arc::new(InMemoryAiProviderRegistry::new());

    let result = composition
        .transform_then_recall_verify(
            providers,
            &MemoryTransformThenRecallRequest {
                transform: MemoryTransformRequest {
                    operation: MemoryTransformOperation::EmbedBackfill,
                    dry_run: true,
                    batch_size: 25,
                    max_nodes: 500,
                    ..Default::default()
                },
                recall: MemoryRecallRequest {
                    query_text: Some("release rollback patterns".to_string()),
                    ..Default::default()
                },
            },
        )
        .await?;

    println!("selected={}, updated={}", result.transform.selected, result.transform.updated);
    println!("retrieved={}", result.recall.retrieved);
    Ok(())
}
```

Prompt template:

```text
system:
You are a release reliability assistant.
Prefer conservative recommendations when regression evidence is incomplete.

user:
Release context: {{release_context}}
Transform preview: {{transform_summary}}
Recall context: {{recall_summary}}

Return:
1) promote / hold / rollback recommendation
2) top risk factors
3) confidence
4) mandatory verification checks
```

## Cookbook 4: Research Synthesizer (Deterministic Memory Build)

Intent:

1. Structure raw research notes into deterministic memory content.
2. Ask the model to synthesize only after context is structured and scoped.

```rust
use anyhow::Result;
use std::sync::Arc;
use locus_core::{InMemoryNodeStore, NodeStore};
use locus_sdk::prelude::{
    CompositeInputItem, CompositeNodeFromTextOptions, CompositeNodeFromTextRequest,
    CompositeRole, MemoryCompositionService,
};

fn main() -> Result<()> {
    let store: Arc<dyn NodeStore> = Arc::new(InMemoryNodeStore::new());
    let composition = MemoryCompositionService::new(store);

    let structured = composition.build_content_from_text(&CompositeNodeFromTextRequest {
        items: vec![CompositeInputItem {
            role: CompositeRole::Document,
            text: "Policy memo highlights housing supply bottlenecks".to_string(),
            avec_override: None,
            context: vec![],
        }],
        options: CompositeNodeFromTextOptions {
            allow_llm_avec_fallback: false,
            max_recursion_depth: 3,
            ..Default::default()
        },
    })?;

    println!("resolved_avec_count={}", structured.resolved_avec_count);
    println!("requires_llm_avec={}", structured.requires_llm_avec);
    Ok(())
}
```

Prompt template:

```text
system:
You are a research synthesis assistant.
Preserve uncertainty and disagreement; do not collapse conflicting evidence.

user:
Question: {{research_question}}
Structured context payload:
{{structured_content}}

Return:
1) synthesis
2) competing interpretations
3) confidence per claim
4) additional evidence needed
```

## Prompting Rules That Work Well With STTP

1. Require evidence-cited outputs for high-impact decisions.
2. Ask for explicit confidence fields rather than vague certainty language.
3. Pass retrieval metadata to the prompt when using explain-driven guardrails.
4. Preserve contradictions in output instead of forcing a single narrative.

## Anti-Patterns

1. Letting model responses bypass retrieval and explain checks.
2. Running transforms without dry-run in fresh environments.
3. Using broad session scopes that mix unrelated workflows.
4. Tuning alpha and beta without checking retrieval_path drift.

## Where To Go Next

1. For domain presets, continue with [Agent Blueprints by Domain](agent-blueprints.md).
2. For API-level examples, use [Examples](examples.md).
3. For rollout safeguards, use [Integration](integration.md) and [Operations](operations.md).