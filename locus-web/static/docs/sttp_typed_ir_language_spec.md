# STTP Typed IR Language Specification
## Formal Contract for Cognitive State Transfer

Status: Draft
Version: 1.1.0-draft
Last Updated: 2026-04-25

## 1. Purpose and Position
STTP defines a typed intermediate representation for cognition state transfer between agents.

This specification defines:
1. the language surface,
2. the structural and semantic contract,
3. conformance profiles,
4. security and verification obligations.

This specification does not define:
1. storage backend implementation details,
2. sync authority policy,
3. retrieval ranking policy.

## 2. Normative Keywords
Keywords have normative meaning:
1. MUST and MUST NOT indicate mandatory requirements.
2. SHOULD and SHOULD NOT indicate strong recommendations.
3. MAY indicates optional behavior.

## 3. Conceptual Model
An STTP node is a four-layer typed artifact with ordered semantics.

Required semantic order:
1. Provenance: orientation and transmission contract.
2. Envelope: temporal identity and dual actor state.
3. Content: compressed meaning with confidence typing.
4. Metrics: quality and compression state verification.

Layer order is part of meaning. Reordering alters interpretation and invalidates strict conformance.

## 4. Structural Symbols
Reserved protocol symbols:
1. Node marker: ⏣
2. Provenance marker: ⊕⟨
3. Envelope marker: ⦿⟨
4. Content marker: ◈⟨
5. Metrics marker: ⍉⟨
6. Layer stop marker: ⟩

Implementations MUST treat these tokens as structural control symbols when encountered in structural positions.

## 5. Lexical Domain
### 5.1 Required Token Classes
A conforming lexer MUST recognize:
1. protocol markers,
2. braces, brackets, and parentheses,
3. delimiters,
4. identifiers,
5. numeric literals,
6. string literals,
7. boolean and null literals.

### 5.2 Identifiers
Canonical identifier regex:
- [A-Za-z_][A-Za-z0-9_]*

### 5.3 Scalars and Containers
Scalar values:
1. String
2. Number
3. Boolean
4. Null

Container values:
1. Object
2. Array

## 6. Grammar Skeleton
The following grammar is normative at the layer level.

```text
Node             := ProvenanceLayer EnvelopeLayer ContentLayer MetricsLayer

ProvenanceLayer  := PROVENANCE_START NodeObject LAYER_END
EnvelopeLayer    := ENVELOPE_START   NodeObject LAYER_END
ContentLayer     := CONTENT_START    NodeObject LAYER_END
MetricsLayer     := METRICS_START    NodeObject LAYER_END

NodeObject       := OptionalNodeMarker Object
OptionalNodeMarker := NODE_MARKER [NodeOrdinal]
NodeOrdinal      := Integer

Object           := '{' Members? '}'
Members          := Pair (',' Pair)*
Pair             := Key ':' Value
Key              := Identifier | String
Value            := Object | Array | String | Number | Boolean | Null
Array            := '[' (Value (',' Value)*)? ']'
```

Note:
1. legacy compact separators are non-canonical and profile-scoped to tolerant mode.

## 7. Layer Schema Contract
### 7.1 Provenance Layer
Required keys:
1. trigger
2. response_format
3. origin_session
4. compression_depth
5. parent_node
6. prime

trigger enum:
1. scheduled
2. threshold
3. resonance
4. seed
5. manual

response_format enum:
1. temporal_node
2. natural_language
3. hybrid

prime required keys:
1. attractor_config
2. context_summary
3. relevant_tier
4. retrieval_budget

attractor_config required keys:
1. stability
2. friction
3. logic
4. autonomy

### 7.2 Envelope Layer
Required keys:
1. timestamp
2. tier
3. session_id
4. user_avec
5. model_avec

Optional keys:
1. schema_version

tier enum:
1. raw
2. daily
3. weekly
4. monthly
5. quarterly
6. yearly

AVEC required keys:
1. stability
2. friction
3. logic
4. autonomy
5. psi

### 7.3 Content Layer
Required shape:
1. typed object containing one or more semantic fields.

Canonical field signature:
1. field_name(.confidence): value

confidence rules:
1. confidence MUST parse as float.
2. strict range is [0.0, 1.0].

depth rule:
1. strict maximum nesting depth is 5.

### 7.4 Metrics Layer
Required keys:
1. rho
2. kappa
3. psi
4. compression_avec

compression_avec required keys:
1. stability
2. friction
3. logic
4. autonomy
5. psi

## 8. Semantic Invariants
Strict conformance requires all invariants below.

S1 Layer order invariant:
1. provenance precedes envelope.
2. envelope precedes content.
3. content precedes metrics.

S2 Completeness invariant:
1. all required keys exist in all required layers.

S3 Type invariant:
1. enum values are members of defined enum sets.
2. required numeric fields parse as numeric values.

S4 Confidence invariant:
1. every canonical content field includes confidence annotation.
2. every confidence annotation is range-valid in strict mode.

S5 Structural depth invariant:
1. content object nesting depth is less than or equal to 5.

A node that violates any invariant MUST be marked strict-invalid.

## 9. Compliance Profiles
### 9.1 Strict Profile
Strict profile MUST:
1. enforce all syntax and invariants,
2. fail closed on unresolved structure,
3. reject non-canonical legacy forms.

### 9.2 Tolerant Profile
Tolerant profile MAY accept legacy forms including:
1. compact pipe-delimited key-value payloads,
2. omitted internal node wrappers,
3. unquoted enum-like values where unambiguous,
4. mixed formatting and spacing variants.

Tolerant profile MUST:
1. preserve raw source,
2. emit diagnostics for every deviation,
3. produce canonical AST when recoverable,
4. never claim strict-valid unless strict invariants pass.

## 10. Diagnostics Specification
Recommended severities:
1. Fatal
2. Error
3. Warning
4. Info

Diagnostic payload SHOULD include:
1. code,
2. message,
3. source span,
4. recovery action,
5. strict impact.

## 11. Canonicalization Rules
When tolerant recovery is used, implementation SHOULD canonicalize:
1. layer ordering in AST output,
2. known aliases into canonical keys,
3. parent_node into normalized representation,
4. unambiguous numeric strings into numeric values.

Canonicalization MUST NOT:
1. modify original raw text,
2. alter identity-sensitive semantics,
3. fabricate missing semantic values.

## 12. Adversarial and Security Considerations
Threat classes:
1. marker spoofing and layer confusion,
2. malformed confidence signatures,
3. delimiter smuggling in compact forms,
4. layer bleed-through via unbalanced containers.

Required response:
1. strict profile fails closed,
2. tolerant profile fails informative with bounded recovery,
3. diagnostics remain deterministic for equal input.

## 13. Backward Compatibility Contract
Language evolution policy is additive.

Rules:
1. optional fields MAY be introduced without invalidating old nodes.
2. old nodes MAY be ingested via tolerant profile.
3. strict profile remains the canonical generation target.

## 14. Conformance Testing Requirements
A conformance suite SHOULD include:
1. strict canonical fixtures,
2. tolerant legacy fixtures,
3. negative fixtures by invariant class,
4. adversarial fixtures by threat class,
5. canonical AST snapshot expectations.

## 15. Canonical Example
```text
⊕⟨ ⏣0{ trigger: manual, response_format: temporal_node, origin_session: "session-abc", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.90, friction: 0.20, logic: 0.98, autonomy: 0.85 }, context_summary: "parser hardening session", relevant_tier: raw, retrieval_budget: 8 } } ⟩
⦿⟨ ⏣0{ timestamp: "2026-04-25T00:00:00Z", tier: raw, session_id: "session-abc", schema_version: "sttp-1.0", user_avec: { stability: 0.90, friction: 0.20, logic: 0.98, autonomy: 0.85, psi: 2.93 }, model_avec: { stability: 0.90, friction: 0.20, logic: 0.98, autonomy: 0.85, psi: 2.93 } } ⟩
◈⟨ ⏣0{ focus(.99): "grammar update", decision(.96): { parser_mode(.95): "strict_and_tolerant" } } ⟩
⍉⟨ ⏣0{ rho: 0.95, kappa: 0.94, psi: 2.93, compression_avec: { stability: 0.90, friction: 0.20, logic: 0.98, autonomy: 0.85, psi: 2.93 } } ⟩
```

## 16. Change Control
When schema or grammar rules change, maintainers SHOULD update:
1. this language specification,
2. sttp_grammar_blueprint.md,
3. parser conformance fixtures,
4. parser implementation notes.

## 17. Language Promise
STTP as typed IR is complete only when it can do all three simultaneously:
1. represent cognition state faithfully,
2. validate structure deterministically,
3. survive historical reality without semantic drift.
