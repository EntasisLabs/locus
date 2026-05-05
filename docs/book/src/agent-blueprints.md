# Agent Blueprints by Domain

## Audience and Use

This page is for teams designing task-specific agents that share one STTP memory backbone.

Use it to:

1. Map business tasks to concrete agent memory behavior.
2. Reuse reliable STTP patterns across multiple domains.
3. Start with practical defaults before deep customization.

## Blueprint Format

Each blueprint defines:

1. Primary task and success criteria.
2. Memory ingestion pattern across raw, daily, and weekly tiers.
3. Retrieval policy defaults.
4. Operational guardrails and handoff points.

## Shared Foundation for All Blueprints

Start each agent with these defaults:

1. Parse and validate all stored nodes before persistence.
2. Require explicit tenant and session scope on write and recall.
3. Store key runtime policy values alongside each operation.
4. Keep explain-capable recall enabled for incident diagnostics.

## Blueprint 1: Customer Support Resolution Agent

Primary task:

1. Resolve customer issues faster while preserving account context.

Memory pattern:

1. raw: ticket transcript fragments, action decisions, workaround attempts.
2. daily: condensed issue timeline and resolution outcomes.
3. weekly: recurring issue clusters and escalation themes.

Retrieval defaults:

1. Start with hybrid retrieval when embeddings exist.
2. Enable lexical fallback for sparse or new accounts.
3. Limit recall result set to a small operator-reviewable window.

Guardrails:

1. Escalate when confidence on root-cause fields falls below policy threshold.
2. Require human approval before customer-visible policy exceptions.

## Blueprint 2: Revenue and Account Strategy Agent

Primary task:

1. Prepare account intelligence summaries for sales and success teams.

Memory pattern:

1. raw: call notes, objections, product-fit signals, next-step commitments.
2. daily: account momentum summary with confidence annotations.
3. weekly: pipeline risk and expansion opportunity themes.

Retrieval defaults:

1. Prioritize recent daily and weekly tiers for executive summaries.
2. Pull raw evidence snippets for every high-impact recommendation.

Guardrails:

1. Block outbound recommendations lacking supporting evidence links.
2. Flag contradictory account signals for manual review.

## Blueprint 3: Security Incident Triage Agent

Primary task:

1. Speed investigation and containment decisions during active incidents.

Memory pattern:

1. raw: alerts, timeline events, containment steps, IOC references.
2. daily: incident narrative with mitigation state.
3. weekly: recurring attack pattern and control-gap rollups.

Retrieval defaults:

1. Bias toward strict time-window filtering during active incidents.
2. Use explain flows on every severity upgrade recommendation.

Guardrails:

1. Require dual confirmation before irreversible containment actions.
2. Freeze autonomous remediation if retrieval path deviates from baseline.

## Blueprint 4: Clinical Operations Coordination Agent

Primary task:

1. Improve handoff quality across clinical operations teams.

Memory pattern:

1. raw: triage notes, operational handoff details, care coordination tasks.
2. daily: shift-level summary and unresolved coordination items.
3. weekly: recurring workflow bottlenecks and staffing friction themes.

Retrieval defaults:

1. Prefer deterministic recall windows scoped to active handoff queues.
2. Keep retrieval limits tight for auditability and review speed.

Guardrails:

1. Restrict memory access by role and operational scope.
2. Require explicit redaction policy for sensitive fields before storage.

## Blueprint 5: Engineering Release Reliability Agent

Primary task:

1. Reduce regressions by preserving release and rollback context.

Memory pattern:

1. raw: deploy events, test failures, rollback attempts, mitigation notes.
2. daily: release status summary and unresolved risks.
3. weekly: stability trend and recurring failure-class rollup.

Retrieval defaults:

1. Compare current release signals against recent weekly baselines.
2. Use explain traces to justify promote, hold, or rollback decisions.

Guardrails:

1. Require rollback recommendation when failure classes repeat beyond threshold.
2. Block promotion if validation success metrics regress from baseline.

## Blueprint 6: Research and Policy Analysis Agent

Primary task:

1. Build traceable recommendations from large document sets.

Memory pattern:

1. raw: source excerpts with provenance and extraction rationale.
2. daily: topic summaries with confidence-scored claims.
3. weekly: cross-source synthesis with contradiction tracking.

Retrieval defaults:

1. Retrieve source-backed evidence first, synthesized claims second.
2. Use lexical fallback for low-embedding or highly technical terminology.

Guardrails:

1. Do not emit conclusions without source-linked evidence fields.
2. Route high-impact policy conclusions through human review.

## Implementation Template

Use this checklist when instantiating any blueprint:

1. Define task-specific session naming and tenant strategy.
2. Define required content fields and confidence expectations.
3. Define fallback policy and acceptable retrieval_path behavior.
4. Define transform workflows and dry-run policy.
5. Define escalation, rollback, and audit logging requirements.

## Starter Request Shape

Use a request profile like this as a baseline for domain agents:

```text
session scope: <tenant>::<agent-role>::<workstream>
tiers: ["raw", "daily", "weekly"]
retrieval policy:
  mode: hybrid-preferred
  lexical_fallback: enabled
  limit: 5-20 depending on operator review mode
explain: enabled for all high-impact actions
transform mode: dry-run first in new environments
```

## Drop-In Starter Configs

Use these as first-pass defaults. Tune only after collecting baseline metrics.

### Config A: Customer Support Resolution

```text
session pattern: tenant::<account-id>::agent::support-resolution::<ticket-id>
tiers: ["raw", "daily", "weekly"]
limit: 8
alpha: 0.70
beta: 0.30
retrieval mode: hybrid-preferred
lexical fallback: enabled
time window: last 30 days by default
explain: required for escalation and exception decisions
transform mode: dry-run for new policy revisions
```

### Config B: Revenue and Account Strategy

```text
session pattern: tenant::<account-id>::agent::revenue-strategy::<quarter>
tiers: ["daily", "weekly", "raw"]
limit: 12
alpha: 0.60
beta: 0.40
retrieval mode: hybrid-preferred
lexical fallback: enabled
time window: last 90 days by default
explain: required for outbound recommendation generation
transform mode: dry-run for scoring-model changes
```

### Config C: Security Incident Triage

```text
session pattern: tenant::<org-id>::agent::security-triage::<incident-id>
tiers: ["raw", "daily"]
limit: 6
alpha: 0.75
beta: 0.25
retrieval mode: hybrid-preferred with strict time filtering
lexical fallback: enabled
time window: incident start to now
explain: required for severity upgrade and containment recommendations
transform mode: dry-run first, then controlled batch execution
```

### Config D: Clinical Operations Coordination

```text
session pattern: tenant::<facility-id>::agent::clinical-ops::<shift-id>
tiers: ["raw", "daily"]
limit: 7
alpha: 0.65
beta: 0.35
retrieval mode: deterministic-first with hybrid fallback
lexical fallback: enabled
time window: active shift plus previous shift handoff window
explain: required for unresolved handoff item recommendations
transform mode: dry-run until handoff schema is stable
```

### Config E: Engineering Release Reliability

```text
session pattern: tenant::<org-id>::agent::release-reliability::<release-id>
tiers: ["raw", "daily", "weekly"]
limit: 10
alpha: 0.68
beta: 0.32
retrieval mode: hybrid-preferred
lexical fallback: enabled
time window: release window plus previous two release cycles
explain: required for promote, hold, and rollback recommendations
transform mode: dry-run for migration and backfill operations
```

### Config F: Research and Policy Analysis

```text
session pattern: tenant::<program-id>::agent::policy-analysis::<topic-id>
tiers: ["raw", "daily", "weekly"]
limit: 14
alpha: 0.55
beta: 0.45
retrieval mode: hybrid-preferred with source-first ordering
lexical fallback: enabled
time window: topic-defined research horizon
explain: required for all high-impact conclusions
transform mode: dry-run for synthesis template updates
```

## Tuning Sequence

When adjusting any config:

1. Tune limit first to match operator review capacity.
2. Tune alpha and beta second to control resonance versus semantic emphasis.
3. Tune time window third to reduce noise without losing critical context.
4. Keep explain enabled while tuning to verify retrieval_path behavior.

## End-to-End Sample Flows

Each flow uses the same four-step runtime pattern:

1. Ingest: store raw node events.
2. Recall: retrieve relevant context using scoped policy.
3. Explain: inspect retrieval path and score behavior.
4. Transform dry-run: preview rollups or embedding updates before mutation.

### Flow A: Customer Support Resolution

```text
step 1 ingest
  session_id: tenant::acme::agent::support-resolution::ticket-1042
  tier: raw
  content focus: issue summary, attempted fix, customer impact

step 2 recall
  tiers: ["raw", "daily", "weekly"]
  limit: 8
  alpha: 0.70
  beta: 0.30
  lexical fallback: enabled

step 3 explain
  verify retrieval_path is hybrid or lexical_fallback as expected
  confirm top results include prior resolutions for same issue family

step 4 transform dry-run
  operation: monthly_rollup
  dry_run: true
  expected: projected counts and no persisted mutations
```

### Flow B: Revenue and Account Strategy

```text
step 1 ingest
  session_id: tenant::acme::agent::revenue-strategy::2026-q2
  tier: raw
  content focus: objection notes, expansion signals, decision stakeholders

step 2 recall
  tiers: ["daily", "weekly", "raw"]
  limit: 12
  alpha: 0.60
  beta: 0.40
  lexical fallback: enabled

step 3 explain
  verify top recommendations are linked to supporting evidence
  verify contradictory signals are surfaced in results

step 4 transform dry-run
  operation: weekly_rollup
  dry_run: true
  expected: account momentum summary preview without writes
```

### Flow C: Security Incident Triage

```text
step 1 ingest
  session_id: tenant::acme-sec::agent::security-triage::inc-8891
  tier: raw
  content focus: alert details, IOC artifacts, containment actions

step 2 recall
  tiers: ["raw", "daily"]
  limit: 6
  alpha: 0.75
  beta: 0.25
  lexical fallback: enabled
  time window: incident_start..now

step 3 explain
  verify severity recommendation is traceable to matching indicators
  verify retrieval_path remains stable across repeated calls

step 4 transform dry-run
  operation: embed_backfill
  dry_run: true
  expected: selected node count for embedding without updates
```

### Flow D: Clinical Operations Coordination

```text
step 1 ingest
  session_id: tenant::north-hospital::agent::clinical-ops::shift-2026-05-05-n1
  tier: raw
  content focus: handoff notes, unresolved tasks, time-critical blockers

step 2 recall
  tiers: ["raw", "daily"]
  limit: 7
  alpha: 0.65
  beta: 0.35
  lexical fallback: enabled
  time window: active_shift_plus_prior_handoff

step 3 explain
  verify unresolved handoff items rank ahead of historical summaries
  verify retrieval_path reflects deterministic-first policy intent

step 4 transform dry-run
  operation: daily_rollup
  dry_run: true
  expected: shift summary preview for coordinator review
```

### Flow E: Engineering Release Reliability

```text
step 1 ingest
  session_id: tenant::acme-eng::agent::release-reliability::release-2.7.0
  tier: raw
  content focus: test failures, deploy checkpoints, rollback notes

step 2 recall
  tiers: ["raw", "daily", "weekly"]
  limit: 10
  alpha: 0.68
  beta: 0.32
  lexical fallback: enabled
  time window: current_release_plus_two_prior

step 3 explain
  verify recurring failure classes are present in top-ranked context
  verify promote or rollback suggestion is evidence-backed

step 4 transform dry-run
  operation: monthly_rollup
  dry_run: true
  expected: release stability trend preview with no writes
```

### Flow F: Research and Policy Analysis

```text
step 1 ingest
  session_id: tenant::policy-lab::agent::policy-analysis::housing-supply
  tier: raw
  content focus: source excerpts, claims, contradiction annotations

step 2 recall
  tiers: ["raw", "daily", "weekly"]
  limit: 14
  alpha: 0.55
  beta: 0.45
  lexical fallback: enabled
  ordering intent: source-first then synthesis

step 3 explain
  verify every high-impact conclusion maps to source-backed entries
  verify contradiction nodes are retained in top context window

step 4 transform dry-run
  operation: weekly_rollup
  dry_run: true
  expected: synthesis preview with confidence-bearing claim structure
```

## Fast Adaptation Notes

1. Keep the four-step flow constant across domains.
2. Change session naming and time windows first for domain fit.
3. Change alpha and beta only after baseline explain traces are stable.
4. Keep transform in dry-run mode until rollback criteria are documented.

## Choosing Your First Blueprint

1. High-volume interaction workflows: start with Customer Support Resolution.
2. Time-critical response workflows: start with Security Incident Triage.
3. Cross-team handoff workflows: start with Clinical Operations Coordination.
4. Decision-justification workflows: start with Research and Policy Analysis.

Then adapt the same STTP patterns to your domain-specific contracts and controls.
