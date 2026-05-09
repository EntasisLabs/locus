# locus-mcp

Language models are stateless between chats. `locus-mcp` gives that state somewhere to go.

Rust MCP server for STTP memory operations, built on `rmcp` and `locus-core-rs`.

This server runs over stdio and currently exposes nine tools:

- `get_schema`
- `calibrate_session`
- `store_context`
- `get_context`
- `list_nodes`
- `preview_embedding_migration`
- `run_embedding_migration`
- `get_moods`
- `create_monthly_rollup`

## Why This Exists

- Session memory continuity: store compressed context and rehydrate it later.
- Cross-session retrieval: recover relevant state from AVEC resonance.
- Schema-first safety: strict typed-IR ingest profile by default.
- Structured rollups: aggregate raw memory into periodic checkpoints.

## What It Supports

- In-memory mode for local smoke tests.
- SurrealDB-backed mode for persistent memory.
- Configurable parser profiles for ingest strictness:
  - `strict_typed_ir` (default)
  - `strict`
  - `tolerant`
- Optional global retrieval (`get_context` with no `session_id`).
- Optional retrieval filters:
  - time window (`from_utc`, `to_utc`)
  - tier filter (`tiers`)
- Optional hybrid retrieval in `get_context` when `context_keywords` is provided.
- Optional auto-embedding on `store_context` with selectable providers:
  - Ollama (default provider)
  - Local embedding (feature-gated)

## Requirements

- Docker
- For persistent mode:
  - reachable SurrealDB endpoint
  - namespace/database/user/password

## First 5 Minutes

If you are new to this repo, this path gives a fast first success:

1. Start in-memory mode with strict typed IR using GHCR image:

```bash
docker run --rm -i \
  -e LOCUS_MCP_IN_MEMORY=true \
  -e LOCUS_MCP_PARSE_PROFILE=strict_typed_ir \
  ghcr.io/entasislabs/locus-mcp:<version>
```

2. In your MCP client, call `get_schema`.
3. Copy the returned canonical node pattern and call `store_context` with a valid payload.
4. Call `get_context` using the same `session_id` and AVEC values.

If that works, your end-to-end loop is healthy and you can move to persistent SurrealDB mode.

## Quick Start

Run the standard image (no local embeddings):

```bash
docker run --rm -i \
  -e LOCUS_MCP_IN_MEMORY=true \
  -e LOCUS_MCP_PARSE_PROFILE=strict_typed_ir \
  ghcr.io/entasislabs/locus-mcp:<version>
```

Run the embeddings image (recommended for local RAG embeddings):

```bash
docker run --rm -i \
  -e LOCUS_MCP_IN_MEMORY=true \
  -e LOCUS_MCP_PARSE_PROFILE=strict_typed_ir \
  ghcr.io/entasislabs/locus-mcp-embeddings:<version>
```

Run with SurrealDB remote mode (example):

```bash
docker run --rm -i \
  -e LOCUS_MCP_REMOTE=true \
  -e LOCUS_MCP_PARSE_PROFILE=strict_typed_ir \
  -e LOCUS_MCP_SURREAL_REMOTE_ENDPOINT=ws://127.0.0.1:8000/rpc \
  -e LOCUS_MCP_SURREAL_NAMESPACE=entasis \
  -e LOCUS_MCP_SURREAL_DATABASE=locus_mcp \
  -e LOCUS_MCP_SURREAL_USERNAME=root \
  -e LOCUS_MCP_SURREAL_PASSWORD=root \
  ghcr.io/entasislabs/locus-mcp:<version>
```

## Configuration

The server resolves storage mode and connection settings from env vars and optional CLI args.

### Storage Mode Selection

- `LOCUS_MCP_IN_MEMORY=true` enables in-memory mode.
- `LOCUS_MCP_STORAGE=inmemory` also enables in-memory mode.
- `--in-memory` also enables in-memory mode.
- Otherwise server defaults to SurrealDB mode.

### SurrealDB Connection Inputs

Env vars:

- `LOCUS_MCP_REMOTE=true|false`
- `LOCUS_MCP_SURREAL_REMOTE_ENDPOINT`
- `LOCUS_MCP_SURREAL_EMBEDDED_ENDPOINT`
- `LOCUS_MCP_SURREAL_ENDPOINT` (applies to both remote and embedded slots)
- `LOCUS_MCP_SURREAL_NAMESPACE`
- `LOCUS_MCP_SURREAL_DATABASE`
- `LOCUS_MCP_SURREAL_USERNAME`
- `LOCUS_MCP_SURREAL_PASSWORD`

CLI alternatives:

- `--remote-endpoint <value>`
- `--embedded-endpoint <value>`
- `--endpoint <value>`
- `--namespace <value>`
- `--database <value>`
- `--username <value>`
- `--password <value>`
- `--remote`

Notes:

- If `LOCUS_MCP_REMOTE=true`, `--remote` is injected for runtime resolution.
- If auth values are omitted in remote mode, defaults are `root/root`.

### Embedding Provider Inputs

Env vars:

- `LOCUS_MCP_EMBEDDINGS_ENABLED=true|false`
- `LOCUS_MCP_EMBEDDINGS_PROVIDER` (`ollama` or `local` when built with `local-embedding`)
- `LOCUS_MCP_EMBEDDINGS_ENDPOINT` (used by Ollama provider)
- `LOCUS_MCP_EMBEDDINGS_MODEL`
- `LOCUS_MCP_EMBEDDINGS_REPO` (used by local embedding provider)

CLI alternatives:

- `--embeddings-enabled`
- `--embeddings-provider <ollama|local>`
- `--embeddings-endpoint <value>`
- `--embeddings-model <value>`
- `--embeddings-repo <value>`

Notes:

- Provider defaults to `ollama`.
- If `local` is requested without building with `--features local-embedding`, startup returns a configuration error.
- `store_context` embedding generation is fail-open: node storage still succeeds if embedding generation fails.

### Parse Profile Inputs

Env vars:

- `LOCUS_MCP_PARSE_PROFILE` (`strict_typed_ir`, `strict`, `tolerant`)

CLI alternatives:

- `--parse-profile <strict_typed_ir|strict|tolerant>`

Notes:

- Default profile is `strict_typed_ir` when not set.
- Accepted aliases for strict typed-IR include `strict-typed-ir`, `stricttypedir`, `typed_ir`, and `typed-ir`.
- `strict_typed_ir` is recommended for production-grade schema adherence.

## Tool Reference

All tool outputs are JSON strings.

### `calibrate_session`

Input:

```json
{
  "session_id": "my-session",
  "stability": 0.82,
  "friction": 0.31,
  "logic": 0.88,
  "autonomy": 0.74,
  "trigger": "manual"
}
```

### `store_context`

Input:

```json
{
  "session_id": "my-session",
  "node": "<full STTP node payload>"
}
```

Notes:

- MCP ingest policy is strict typed IR (`profile_policy = strict_typed_ir`).
- For model-friendly retries, call `get_schema` first, then shape payload to the required layered/typed form before calling `store_context` again.

### `get_schema`

Input:

```json
{}
```

Returns the SDK memory capability schema plus MCP ingest policy guidance, including strict typed IR profile and schema-first workflow hints.

### `get_context`

Input (minimum):

```json
{
  "session_id": "my-session",
  "stability": 0.82,
  "friction": 0.31,
  "logic": 0.88,
  "autonomy": 0.74
}
```

Optional fields:

- `session_id` omitted => global retrieval
- `limit` (clamped to `1..200`, default `5`)
- `from_utc`, `to_utc` (ISO8601 datetime)
- `tiers` (array of tier names)
- `context_keywords` (array of strings for semantic/fuzzy retrieval)
- `alpha` and `beta` for hybrid weighting when `context_keywords` is present (defaults `0.7` and `0.3`)

### `list_nodes`

Input:

```json
{
  "limit": 50,
  "session_id": "my-session",
  "context_keywords": ["strict", "parser"]
}
```

- `limit` clamped to `1..200`
- `session_id` optional
- `context_keywords` optional (fuzzy filter on context summary)

### `preview_embedding_migration`

Input (example):

```json
{
  "session_id": "my-session",
  "tiers": ["raw"],
  "has_embedding": false,
  "sample_limit": 20,
  "max_nodes": 5000
}
```

Use this to inspect migration scope before running any embedding backfill.

### `run_embedding_migration`

Input (example):

```json
{
  "session_id": "my-session",
  "mode": "missing_only",
  "dry_run": true,
  "batch_size": 100,
  "max_nodes": 5000
}
```

Set `dry_run` to `false` to apply updates.

### `get_moods`

Input:

```json
{
  "target_mood": "analytical",
  "blend": 0.7,
  "current_stability": 0.6,
  "current_friction": 0.4,
  "current_logic": 0.8,
  "current_autonomy": 0.7
}
```

All fields are optional except `blend` defaults to `1.0`.

### `create_monthly_rollup`

Input:

```json
{
  "session_id": "my-session",
  "start_date_utc": "2026-04-01T00:00:00Z",
  "end_date_utc": "2026-04-30T23:59:59Z",
  "source_session_id": "my-session",
  "parent_node_id": null,
  "persist": true
}
```

## VS Code Setup (Docker MCP)

This server is designed to run over stdio, so VS Code can launch it directly as an MCP server.

### 1) Choose config scope

Use either:

- Workspace-level MCP config (recommended for repo-local setup)
- User-level MCP config (if you want it available across projects)

If your VS Code MCP UI created a config file already, use that file and add the server entry below.

### 2) Add server entry

Known-good minimal config for first run:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "-e",
        "LOCUS_MCP_IN_MEMORY=true",
        "-e",
        "LOCUS_MCP_PARSE_PROFILE=strict_typed_ir",
        "ghcr.io/entasislabs/locus-mcp:<version>"
      ]
    }
  }
}
```

Use the embeddings image for local RAG-oriented setups:

```json
{
  "mcpServers": {
    "locus-mcp-embeddings": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "-e",
        "LOCUS_MCP_IN_MEMORY=true",
        "-e",
        "LOCUS_MCP_PARSE_PROFILE=strict_typed_ir",
        "ghcr.io/entasislabs/locus-mcp-embeddings:<version>"
      ]
    }
  }
}
```

Persistent SurrealDB variant:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "-e",
        "LOCUS_MCP_REMOTE=true",
        "-e",
        "LOCUS_MCP_PARSE_PROFILE=strict_typed_ir",
        "-e",
        "LOCUS_MCP_SURREAL_REMOTE_ENDPOINT=ws://127.0.0.1:8000/rpc",
        "-e",
        "LOCUS_MCP_SURREAL_NAMESPACE=entasis",
        "-e",
        "LOCUS_MCP_SURREAL_DATABASE=locus_mcp",
        "-e",
        "LOCUS_MCP_SURREAL_USERNAME=root",
        "-e",
        "LOCUS_MCP_SURREAL_PASSWORD=root",
        "ghcr.io/entasislabs/locus-mcp:<version>"
      ]
    }
  }
}
```

For early exploration and lenient payload experiments, switch the profile:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "-e",
        "LOCUS_MCP_IN_MEMORY=true",
        "-e",
        "LOCUS_MCP_PARSE_PROFILE=tolerant",
        "ghcr.io/entasislabs/locus-mcp:<version>"
      ]
    }
  }
}
```

### 3) Reload and verify in VS Code

After saving MCP config:

- Reload VS Code window.
- Open MCP tools in chat.
- Confirm `locus-mcp` appears.
- Run a first tool call such as `get_moods` or `calibrate_session`.

## First Live Test Flow

After wiring the MCP server:

1. Call `get_schema` and start from the canonical layered node example.
2. Call `calibrate_session` for a test session.
3. Call `store_context` with one valid STTP node.
4. Call `get_context` with matching AVEC values and `session_id` set.
5. Call `list_nodes` for the same `session_id`.
6. Optionally call `get_context` again without `session_id` to confirm global mode.

## Troubleshooting

- `StrictTypedIrParseFailure` or strict profile rejections:
  - Call `get_schema` first and rebuild payload from the canonical layered shape.
  - Confirm required keys and enum/numeric fields are present and valid.
- `InvalidDate` errors:
  - Ensure `from_utc` and `to_utc` are valid ISO8601 timestamps.
- Empty retrieval:
  - Verify the node stored successfully and query AVEC values are reasonable.
  - Remove filters (`tiers`, dates) to verify baseline retrieval.
- Surreal connection/auth failures:
  - Double-check endpoint, namespace/database, and credentials.
- Docker launch failures:
  - Confirm Docker is installed and running.
  - Pull the image manually once to verify access:
    - `docker pull ghcr.io/entasislabs/locus-mcp:<version>`
    - `docker pull ghcr.io/entasislabs/locus-mcp-embeddings:<version>`

## Development Notes

- Logging goes to stderr via `tracing_subscriber`.
- Server transport is stdio via `rmcp`.
- Tool return payloads are serialized as JSON strings by design.
