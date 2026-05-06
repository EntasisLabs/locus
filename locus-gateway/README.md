# locus-gateway

Rust STTP gateway exposing a unified HTTP + gRPC surface over `locus-core-rs`.

It is intended to be deployable as a single binary in front of STTP storage backends.

Like the C# gateway, it is meant to stay easy to reason about: it exposes STTP over network transports and reuses the Rust core for parsing, storage, retrieval, and sync-ready storage semantics.

## What It Provides

- HTTP API using axum.
- gRPC API using tonic with reflection enabled.
- Backend selection:
  - in-memory backend for local/dev scenarios.
  - SurrealDB backend for persistent deployments.
- Multi-tenant scoping with backward-compatible default tenant behavior.
- Batch scope rekey operation (dry-run + apply), anchored by node IDs.
- Sync-ready storage primitives under the hood, without forcing application-specific sync policy.

## What "Sync-Ready" Means Here

The Rust core can now support:

- deterministic node identity for synchronization
- incremental change queries
- checkpoint persistence per connector
- typed connector metadata

But this gateway does not assume your cloud/local strategy for you.

If you only want a deployable STTP API, that is still the default experience. The extra sync pieces are there to support future application-level sync implementations, not to make normal gateway usage more complicated.

## Quick Start

Run with defaults (in-memory backend, HTTP 8080, gRPC 8081):

```bash
cargo run --manifest-path locus-gateway/Cargo.toml
```

Run with Surreal remote backend:

```bash
cargo run --manifest-path locus-gateway/Cargo.toml -- \
  --backend surreal \
  --remote \
  --surreal-remote-endpoint ws://127.0.0.1:8000/rpc \
  --surreal-namespace entasis \
  --surreal-database locus_gateway \
  --surreal-user root \
  --surreal-password root
```

Important: `--remote` and `--surreal-remote-endpoint` do not change backend mode by themselves.
You must set `--backend surreal` (or `LOCUS_GATEWAY_BACKEND=surreal`) or the gateway will continue using the default `in-memory` backend.

Check health:

```bash
curl -s http://127.0.0.1:8080/health
```

## Runtime Configuration

All flags have environment variable equivalents.

| Flag | Env Var | Default | Notes |
| --- | --- | --- | --- |
| `--http-port` | `LOCUS_GATEWAY_HTTP_PORT` | `8080` | HTTP listener |
| `--grpc-port` | `LOCUS_GATEWAY_GRPC_PORT` | `8081` | gRPC listener (h2c) |
| `--backend` | `LOCUS_GATEWAY_BACKEND` | `in-memory` | `in-memory` or `surreal` |
| `--cors-enabled` | `LOCUS_GATEWAY_CORS_ENABLED` | `true` | Enable/disable HTTP CORS middleware |
| `--cors-allowed-origins` | `LOCUS_GATEWAY_CORS_ALLOWED_ORIGINS` | `*` | `*` or comma-separated absolute origins |
| `--root-dir-name` | `LOCUS_GATEWAY_ROOT_DIR_NAME` | `.locus-gateway` | Local runtime data root name |
| `--remote` | `LOCUS_GATEWAY_REMOTE` | `false` | Surreal remote mode toggle |
| `--surreal-embedded-endpoint` | `LOCUS_SURREAL_EMBEDDED_ENDPOINT` | unset | Embedded endpoint override |
| `--surreal-remote-endpoint` | `LOCUS_SURREAL_REMOTE_ENDPOINT` | unset | Remote endpoint override |
| `--surreal-namespace` | `LOCUS_SURREAL_NAMESPACE` | `entasis` | Surreal namespace |
| `--surreal-database` | `LOCUS_SURREAL_DATABASE` | `locus_gateway` | Surreal database |
| `--surreal-user` | `LOCUS_SURREAL_USER` | `root` | Surreal username |
| `--surreal-password` | `LOCUS_SURREAL_PASSWORD` | `root` | Surreal password |

## Tenant Scoping

### Resolution Rules

- Default tenant is `default`.
- HTTP requests resolve tenant from:
  1. request field (`tenantId`) when present and valid,
  2. one of the supported tenant headers (`x-resonantia-tenant`, `x-tenant-id`, `x-tenant`),
  3. fallback to `default`.
- gRPC requests resolve tenant from metadata keys in this order: `x-resonantia-tenant`, `x-tenant-id`, `x-tenant`, then fallback `default`.

### Session Scoping

- Non-default tenants are internally represented as:

  `tenant:<tenant>::session:<sessionId>`

- Default tenant remains unscoped for backward compatibility.
- API responses normalize and return unscoped `sessionId` values.

### Practical Examples

Store with tenant header:

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/store \
  -H 'content-type: application/json' \
  -H 'x-tenant-id: acme' \
  -d '{
    "sessionId":"gateway-rust-port",
    "node":"⊕⟨ ... ⟩"
  }'
```

List scoped nodes:

```bash
curl -s 'http://127.0.0.1:8080/api/v1/nodes?limit=50&sessionId=gateway-rust-port&tenantId=acme'
```

## HTTP API

### Endpoints

- `GET /health`
- `POST /api/v1/calibrate`
- `POST /api/v1/store`
- `POST /api/store` (legacy alias)
- `POST /store` (legacy alias)
- `POST /api/v1/session/rename`
- `POST /api/session/rename` (legacy alias)
- `POST /session/rename` (legacy alias)
- `POST /api/v1/context`
- `GET /api/v1/nodes?limit=50&sessionId=...&tenantId=...`
- `GET /api/nodes?limit=50&sessionId=...&tenantId=...` (legacy alias)
- `GET /nodes?limit=50&sessionId=...&tenantId=...` (legacy alias)
- `GET /api/v1/graph?limit=1000&sessionId=...&tenantId=...`
- `GET /api/graph?limit=1000&sessionId=...&tenantId=...` (legacy alias)
- `GET /graph?limit=1000&sessionId=...&tenantId=...` (legacy alias)
- `GET /api/v1/moods?targetMood=focused&blend=1`
- `POST /api/v1/rekey`
- `POST /api/v1/rollups/monthly`

### BYO Browser CORS

For no-auth BYO usage from browser clients, CORS is enabled with permissive defaults:

- origins: any
- methods include `GET`, `POST`, `PATCH`, `OPTIONS`
- headers: any (including `Content-Type`, `Authorization`, tenant headers)

Recommended production setup: set explicit origins with `LOCUS_GATEWAY_CORS_ALLOWED_ORIGINS`, for example `https://app.resonantia.ai,http://localhost:5173`.

### Node Store Response Compatibility

- `GET /api/v1/nodes` responses include `syncKey` and `syntheticId`.
- `POST /api/v1/store` responses include:
  - `duplicateSkipped`
  - `upsertStatus` (`created` when valid, otherwise `skipped`)

### Session Rename Contract

- Endpoint: `POST /api/v1/session/rename`
- Aliases: `POST /api/session/rename`, `POST /session/rename`
- Request JSON:

```json
{
  "sourceSessionId": "old-session",
  "targetSessionId": "new-session",
  "allowMerge": false
}
```

- Response JSON:

```json
{
  "sourceSessionId": "old-session",
  "targetSessionId": "new-session",
  "movedNodes": 18,
  "movedCalibrations": 1,
  "scopesApplied": 1
}
```

### Example: Calibrate

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/calibrate \
  -H 'content-type: application/json' \
  -d '{
    "sessionId":"gateway-rust-port",
    "stability":0.9,
    "friction":0.2,
    "logic":0.95,
    "autonomy":0.9,
    "trigger":"manual"
  }'
```

### Example: Context Retrieval

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/context \
  -H 'content-type: application/json' \
  -d '{
    "sessionId":"gateway-rust-port",
    "stability":0.9,
    "friction":0.2,
    "logic":0.95,
    "autonomy":0.9,
    "limit":5
  }'
```

## Batch Rekey (HTTP)

Endpoint: `POST /api/v1/rekey`

Purpose: re-scope data from one tenant/session scope to another across both `temporal_node` and `calibration`, anchored by provided node IDs.

### Request Fields

- `nodeIds`: array of anchor IDs (required).
  - Supports bare node ID (`abc123`) or prefixed form (`temporal_node:abc123`).
- `targetSessionId`: destination session ID (required).
- `targetTenantId`: destination tenant (optional).
  - Falls back to tenant header or default tenant.
- `dryRun`: optional, defaults to `true`.
- `allowMerge`: optional, defaults to `false`.

### Recommended Workflow

1. Call with `dryRun=true`.
2. Inspect `scopes`, `conflictScopes`, and row counts.
3. Re-run with `dryRun=false` only when output looks correct.
4. If target already has rows and merge is intended, set `allowMerge=true`.

### Dry-Run Example

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/rekey \
  -H 'content-type: application/json' \
  -d '{
    "nodeIds": ["05ee92706d2b44e8a040e3db2f58175a"],
    "targetTenantId": "acme",
    "targetSessionId": "gateway-rust-port",
    "dryRun": true,
    "allowMerge": false
  }'
```

### Apply Example

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/rekey \
  -H 'content-type: application/json' \
  -d '{
    "nodeIds": ["05ee92706d2b44e8a040e3db2f58175a"],
    "targetTenantId": "acme",
    "targetSessionId": "gateway-rust-port",
    "dryRun": false,
    "allowMerge": true
  }'
```

## gRPC API

Proto file: `proto/sttp.proto`

Service: `sttp.v1.SttpGatewayService`

Methods:

- `CalibrateSession`
- `StoreContext`
- `GetContext`
- `ListNodes`
- `GetMoods`
- `BatchRekey`
- `CreateMonthlyRollup`

gRPC reflection is enabled.

### grpcurl Examples

List services:

```bash
grpcurl -plaintext 127.0.0.1:8081 list
```

Calibrate with tenant metadata:

```bash
grpcurl -plaintext \
  -H 'x-tenant-id: acme' \
  -d '{"sessionId":"gateway-rust-port","stability":0.9,"friction":0.2,"logic":0.95,"autonomy":0.9,"trigger":"manual"}' \
  127.0.0.1:8081 sttp.v1.SttpGatewayService/CalibrateSession
```

Batch rekey dry-run:

```bash
grpcurl -plaintext \
  -H 'x-tenant-id: acme' \
  -d '{"nodeIds":["05ee92706d2b44e8a040e3db2f58175a"],"targetSessionId":"gateway-rust-port","targetTenantId":"acme","dryRun":true,"allowMerge":false}' \
  127.0.0.1:8081 sttp.v1.SttpGatewayService/BatchRekey
```

## Docker (Host Build + Minimal Runtime Image)

To keep Docker builds fast and cool, this image follows the same pattern as the C# services:

1. Build the binary on the host.
2. Copy publish output into a tiny runtime image.

No Rust toolchain is installed in the container image.

### Prerequisites On Host

- Rust toolchain with cargo.
- Docker.

### One-Command Build And Package

```bash
./locus-gateway/build-image.sh
```

Custom tag:

```bash
./locus-gateway/build-image.sh ghcr.io/entasislabs/locus-gateway:2.0.0
```

### Manual Build And Package

Build locally:

```bash
cargo build --release --locked --manifest-path locus-gateway/Cargo.toml
```

Prepare publish folder:

```bash
mkdir -p locus-gateway/publish
cp locus-gateway/target/release/locus-gateway locus-gateway/publish/locus-gateway
chmod +x locus-gateway/publish/locus-gateway
```

Build image from repository root:

```bash
docker build \
  -f locus-gateway/Dockerfile \
  -t locus-gateway:2.0.0 \
  .
```

### Runtime Image Details

- Base image: ubuntu:24.04.
- Runtime dependency installed: ca-certificates.
- Binary location: /app/locus-gateway.
- Persistent volume: /data.
- HOME is set to /data so embedded storage state persists when /data is mounted.

### Run: In-Memory Backend

```bash
docker run --rm \
  -p 8080:8080 \
  -p 8081:8081 \
  locus-gateway:2.0.0
```

### Run: Surreal Remote Backend

```bash
docker run --rm \
  -p 8080:8080 \
  -p 8081:8081 \
  -e LOCUS_GATEWAY_BACKEND=surreal \
  -e LOCUS_GATEWAY_REMOTE=true \
  -e LOCUS_SURREAL_REMOTE_ENDPOINT=ws://host.docker.internal:8000/rpc \
  -e LOCUS_SURREAL_NAMESPACE=entasis \
  -e LOCUS_SURREAL_DATABASE=locus_gateway \
  -e LOCUS_SURREAL_USER=root \
  -e LOCUS_SURREAL_PASSWORD=root \
  locus-gateway:2.0.0
```

### Run: Embedded Mode With Persistent Volume

```bash
docker run --rm \
  -p 8080:8080 \
  -p 8081:8081 \
  -v locus-gateway-data:/data \
  locus-gateway:2.0.0
```

## Build And Test

```bash
cargo check --manifest-path locus-gateway/Cargo.toml
cargo test --manifest-path locus-gateway/Cargo.toml
```

## Troubleshooting

- If HTTP `404` appears for newly added endpoints, restart the gateway binary to load the latest build.
- If gRPC methods are missing in reflection, rebuild and restart after proto changes.
- If `/api/v1/nodes` is unexpectedly empty while using Surreal flags, verify backend mode is actually `surreal` (`--backend surreal` or `LOCUS_GATEWAY_BACKEND=surreal`).
- If tenant-scoped queries return no data, verify `x-tenant-id`/`tenantId` and session ID pairing.
- For rekey operations, always run dry-run first and inspect conflicts before apply mode.
- If Docker still feels slow, run `build-image.sh` so only packaging runs in Docker and compilation stays on the host cache.
- If container startup fails with `GLIBC_2.38 not found`, rebuild with the updated runtime base (`ubuntu:24.04`) and republish the image tag.
