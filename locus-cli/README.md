# locus-cli

SDK-backed command-line interface for Locus memory operations.

`locus-cli` runs directly on `locus-core` and `locus-sdk` services (no HTTP gateway required).

It is intended as a simple operator-facing surface for the most common workflows:

1. check gateway health
2. calibrate session state
3. store STTP nodes
4. retrieve resonant context
5. inspect node inventory
6. inspect mood presets
7. create monthly rollups

## Build And Run

From repository root:

```bash
cargo run -p locus-cli -- --help
```

Build release artifacts:

```bash
./locus-cli/build.sh
```

Build and upload archives to `locus-cli/vX.Y.Z`:

```bash
./locus-cli/build.sh --publish
```

## Storage Modes

`locus-cli` supports two storage modes:

1. `surreal` (default): persistent storage via SurrealDB endpoint/runtime settings.
2. `in-memory`: ephemeral local memory for testing.

## Global Options

- `--storage` (env: `LOCUS_STORAGE`, default `surreal`)
- `--tenant-id` (env: `LOCUS_TENANT_ID`)
- `--remote` (env: `LOCUS_REMOTE`)
- `--root-dir-name` (env: `LOCUS_ROOT_DIR_NAME`, default `.locus-cli`)
- `--surreal-endpoint` (env: `LOCUS_SURREAL_ENDPOINT`)
- `--surreal-remote-endpoint` (env: `LOCUS_SURREAL_REMOTE_ENDPOINT`)
- `--surreal-embedded-endpoint` (env: `LOCUS_SURREAL_EMBEDDED_ENDPOINT`)
- `--surreal-namespace` (env: `LOCUS_SURREAL_NAMESPACE`, default `entasis`)
- `--surreal-database` (env: `LOCUS_SURREAL_DATABASE`, default `locus_cli`)
- `--surreal-username` (env: `LOCUS_SURREAL_USERNAME`)
- `--surreal-password` (env: `LOCUS_SURREAL_PASSWORD`)
- `--pretty`

## Commands

### Health

```bash
cargo run -p locus-cli -- --storage in-memory health --pretty
```

### Calibrate

```bash
cargo run -p locus-cli -- --storage in-memory calibrate \
  --session-id demo \
  --stability 0.86 \
  --friction 0.18 \
  --logic 0.92 \
  --autonomy 0.74 \
  --trigger manual \
  --pretty
```

### Store

```bash
cargo run -p locus-cli -- --storage in-memory store \
  --session-id demo \
  --node-file ./node.md \
  --pretty
```

### Context

```bash
cargo run -p locus-cli -- --storage in-memory context \
  --session-id demo \
  --stability 0.86 \
  --friction 0.18 \
  --logic 0.92 \
  --autonomy 0.74 \
  --limit 5 \
  --pretty
```

### Nodes

```bash
cargo run -p locus-cli -- --storage in-memory nodes --session-id demo --limit 20 --pretty
```

### Moods

```bash
cargo run -p locus-cli -- --storage in-memory moods --target-mood focused --blend 0.7 --pretty
```

### Rollup

```bash
cargo run -p locus-cli -- --storage in-memory rollup \
  --session-id demo \
  --start-date-utc 2026-05-01T00:00:00Z \
  --end-date-utc 2026-05-31T23:59:59Z \
  --persist true \
  --pretty
```

## Notes

1. `store` expects a file containing one valid STTP node payload.
2. All command output is JSON.
3. CLI runtime env vars are Locus-prefixed and independent from legacy STTP host vars.
