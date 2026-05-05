# Locus Technical Documentation

This book is the technical reference for teams evaluating, integrating, and operating Locus in real systems.

## Who This Is For

- Application developers building products on top of Locus
- Platform engineers integrating memory infrastructure into existing stacks
- Technical teams preparing architecture proposals and implementation plans

## What You Can Decide With This Book

- Whether Locus fits your reliability and integration requirements
- Which runtime shape fits your environment (SDK, gateway, MCP)
- How to implement safely without rewriting contracts later

## What You Get Here

- Architecture and protocol model details
- Storage and schema specifications
- SDK design and composition patterns
- Mini orchestration cookbooks with prompts and composition flows
- Agent blueprint patterns by domain and task model
- Deployment and operations guidance
- Troubleshooting and versioning policy

## API Documentation

Generated Rust API docs are published alongside this book so implementation teams can move from architecture to code-level integration without switching references.

- Local build output: `docs/technical/api/index.html`
- Typical crate roots:
  - `docs/technical/api/locus_core/index.html`
  - `docs/technical/api/locus_sdk/index.html`
  - `docs/technical/api/locus_mcp/index.html`
  - `docs/technical/api/locus_gateway/index.html`
  - `docs/technical/api/locus_cli/index.html`

## Build

From repository root:

```bash
./docs/build-technical-docs.sh
```

Then open:

- `docs/technical/index.html`
