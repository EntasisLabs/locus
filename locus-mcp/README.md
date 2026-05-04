# locus-mcp

Rust MCP server for STTP memory operations, built on `rmcp` and `locus-core`.

This server runs over stdio and exposes six tools:

- `calibrate_session`
- `store_context`
- `get_context`
- `list_nodes`
- `get_moods`
- `create_monthly_rollup`

## What It Supports

- In-memory mode for local smoke tests.
- SurrealDB-backed mode for persistent memory.
- Optional global retrieval (`get_context` with no `session_id`).
- Optional retrieval filters:
  - time window (`from_utc`, `to_utc`)
  - tier filter (`tiers`)
- Optional hybrid retrieval in `get_context` when `query_embedding` is provided.
- Optional auto-embedding on `store_context` with selectable providers:
  - Ollama (default provider)
  - Local embedding (feature-gated)

## Requirements

- Rust toolchain (Cargo)
- For persistent mode:
  - reachable SurrealDB endpoint
  - namespace/database/user/password

## Quick Start

From this directory:

```bash
cargo check -q
```

Run in in-memory mode:

```bash
LOCUS_MCP_IN_MEMORY=true cargo run
```

Run with SurrealDB mode (example):

```bash
LOCUS_MCP_REMOTE=true \
LOCUS_MCP_SURREAL_REMOTE_ENDPOINT=ws://127.0.0.1:8000/rpc \
LOCUS_MCP_SURREAL_NAMESPACE=keryx \
LOCUS_MCP_SURREAL_DATABASE=locus_mcp \
LOCUS_MCP_SURREAL_USERNAME=root \
LOCUS_MCP_SURREAL_PASSWORD=root \
cargo run
```

Run with auto-embedding (Ollama):

```bash
LOCUS_MCP_EMBEDDINGS_ENABLED=true \
LOCUS_MCP_EMBEDDINGS_PROVIDER=ollama \
LOCUS_MCP_EMBEDDINGS_ENDPOINT=http://127.0.0.1:11434/api/embeddings \
LOCUS_MCP_EMBEDDINGS_MODEL=sttp-encoder \
cargo run
```

Run with auto-embedding (local embedding):

```bash
LOCUS_MCP_EMBEDDINGS_ENABLED=true \
LOCUS_MCP_EMBEDDINGS_PROVIDER=local \
LOCUS_MCP_EMBEDDINGS_MODEL=sttp-encoder \
LOCUS_MCP_EMBEDDINGS_REPO=sentence-transformers/all-MiniLM-L6-v2 \
cargo run --features local-embedding
```

Build Docker image with local embedding support:

```bash
LOCUS_MCP_BUILD_FEATURES=local-embedding ./build-image.sh locus-mcp:latest local-embedding
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
- `query_embedding` (array of floats)
- `alpha` and `beta` for hybrid weighting (defaults `0.7` and `0.3`)

### `list_nodes`

Input:

```json
{
  "limit": 50,
  "session_id": "my-session"
}
```

- `limit` clamped to `1..200`
- `session_id` optional

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

## VS Code Setup (Rust MCP)

This server is designed to run over stdio, so VS Code can launch it directly as an MCP server.

### 1) Choose config scope

Use either:

- Workspace-level MCP config (recommended for repo-local setup)
- User-level MCP config (if you want it available across projects)

If your VS Code MCP UI created a config file already, use that file and add the server entry below.

### 2) Add server entry

Use `cargo run` during development.

If VS Code launches MCP commands from your workspace root, this workspace-relative manifest path works:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "locus-mcp/Cargo.toml"
      ],
      "env": {
        "LOCUS_MCP_IN_MEMORY": "true"
      }
    }
  }
}
```

If you prefer an absolute manifest path, use your own repository location:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "/path/to/locus/locus-mcp/Cargo.toml"
      ],
      "env": {
        "LOCUS_MCP_IN_MEMORY": "true"
      }
    }
  }
}
```

### 3) Persistent storage variant (SurrealDB)

Swap the `env` block if you want persistent mode:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "cargo",
      "args": [
        "run",
        "--manifest-path",
        "locus-mcp/Cargo.toml"
      ],
      "env": {
        "LOCUS_MCP_REMOTE": "true",
        "LOCUS_MCP_SURREAL_REMOTE_ENDPOINT": "ws://127.0.0.1:8000/rpc",
        "LOCUS_MCP_SURREAL_NAMESPACE": "keryx",
        "LOCUS_MCP_SURREAL_DATABASE": "locus_mcp",
        "LOCUS_MCP_SURREAL_USERNAME": "root",
        "LOCUS_MCP_SURREAL_PASSWORD": "root"
      }
    }
  }
}
```

### 4) Optional faster startup (compiled binary)

If you prefer not to compile on each launch:

```bash
cd locus-mcp
cargo build --release
```

Then point VS Code to the binary:

```json
{
  "mcpServers": {
    "locus-mcp": {
      "command": "/path/to/locus/locus-mcp/target/release/locus-mcp",
      "env": {
        "LOCUS_MCP_IN_MEMORY": "true"
      }
    }
  }
}
```

### 5) Reload and verify in VS Code

After saving MCP config:

- Reload VS Code window.
- Open MCP tools in chat.
- Confirm `locus-mcp` appears.
- Run a first tool call such as `get_moods` or `calibrate_session`.

## First Live Test Flow

After wiring the MCP server:

1. Call `calibrate_session` for a test session.
2. Call `store_context` with one valid STTP node.
3. Call `get_context` with matching AVEC values and `session_id` set.
4. Call `list_nodes` for the same `session_id`.
5. Optionally call `get_context` again without `session_id` to confirm global mode.

## Troubleshooting

- `InvalidDate` errors:
  - Ensure `from_utc` and `to_utc` are valid ISO8601 timestamps.
- Empty retrieval:
  - Verify the node stored successfully and query AVEC values are reasonable.
  - Remove filters (`tiers`, dates) to verify baseline retrieval.
- Surreal connection/auth failures:
  - Double-check endpoint, namespace/database, and credentials.
- Build failures from workspace root:
  - Ensure commands point to this crate or use `--manifest-path`.

## Development Notes

- Logging goes to stderr via `tracing_subscriber`.
- Server transport is stdio via `rmcp`.
- Tool return payloads are serialized as JSON strings by design.
