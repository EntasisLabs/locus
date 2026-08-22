#!/usr/bin/env bash
# Locus workspace release helper.
#
# Orchestrates preflight checks, artifact builds, and prints tag/publish commands
# for the aligned release line documented below.
#
# Usage:
#   ./scripts/release.sh                 # preflight only (check + test)
#   ./scripts/release.sh --build         # preflight + release artifact builds (dry-run publish)
#   ./scripts/release.sh --build --publish
#   ./scripts/release.sh --images        # build/push container images (requires docker)
#   ./scripts/release.sh --dry-run       # print planned commands without executing builds
#
# Environment:
#   LOCUS_SKIP_TESTS=1    Skip workspace tests during preflight.
#   RUSTC, CARGO_TARGET_DIR  Optional toolchain/target overrides.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Aligned release line (keep in sync with crate Cargo.toml versions).
CORE_VERSION="0.5.0"
SDK_VERSION="0.3.0"
ADAPTER_VERSION="0.1.0"
WASM_VERSION="0.1.0"
GATEWAY_VERSION="0.4.0"
MCP_VERSION="0.3.0"
CLI_VERSION="0.3.0"

DO_BUILD=false
DO_PUBLISH=false
DO_IMAGES=false
DRY_RUN=false

usage() {
  cat <<EOF
Usage: ./scripts/release.sh [options]

Release line:
  locus-core-rs          ${CORE_VERSION}
  locus-sdk              ${SDK_VERSION}
  locus-surreal-adapter  ${ADAPTER_VERSION}
  locus-wasm             ${WASM_VERSION}
  locus-gateway          ${GATEWAY_VERSION}
  locus-mcp              ${MCP_VERSION}
  locus-cli              ${CLI_VERSION}

Options:
  --build       Run ./build.sh release artifacts after preflight
  --publish     Forward --publish to ./build.sh (GitHub releases / crates.io for core)
  --images      Build service Docker images via ./build.sh --mode images
  --dry-run     Print build commands without executing them
  -h, --help    Show this help
EOF
}

run() {
  if $DRY_RUN; then
    printf '[DRY-RUN]'
    printf ' %q' "$@"
    echo
    return 0
  fi
  "$@"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build) DO_BUILD=true; shift ;;
    --publish) DO_PUBLISH=true; shift ;;
    --images) DO_IMAGES=true; shift ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "error: unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
done

echo "[release] Locus aligned release line"
echo "  core          ${CORE_VERSION}"
echo "  sdk           ${SDK_VERSION}"
echo "  surreal-adpt  ${ADAPTER_VERSION}"
echo "  wasm          ${WASM_VERSION}"
echo "  gateway       ${GATEWAY_VERSION}"
echo "  mcp           ${MCP_VERSION}"
echo "  cli           ${CLI_VERSION}"
echo

echo "[release] Preflight: cargo check --workspace"
run cargo check --workspace --manifest-path "$REPO_ROOT/Cargo.toml"

if [[ "${LOCUS_SKIP_TESTS:-0}" != "1" ]]; then
  echo "[release] Preflight: cargo test (core, sdk, adapter, gateway, mcp, cli)"
  run cargo test -p locus-core-rs -p locus-sdk -p locus-surreal-adapter -p locus-gateway -p locus-mcp -p locus-cli
else
  echo "[release] Skipping tests (LOCUS_SKIP_TESTS=1)"
fi

if $DO_BUILD; then
  build_args=(
    --mode release
    --targets core,mcp,gateway,cli
    --mcp-version "$MCP_VERSION"
    --gateway-version "$GATEWAY_VERSION"
    --cli-version "$CLI_VERSION"
  )
  if $DO_PUBLISH; then
    build_args+=(--publish)
  fi
  if $DRY_RUN; then
    build_args+=(--dry-run)
  fi
  echo "[release] Building release artifacts"
  run "$REPO_ROOT/build.sh" "${build_args[@]}"
  echo "[release] crates.io publish order (manual after core is live):"
  echo "  ./locus-core-rs/publish-crates.sh --publish"
  echo "  ./locus-surreal-adapter/publish-crates.sh --publish"
  echo "  ./locus-sdk/publish-crates.sh --publish"
fi

if $DO_IMAGES; then
  image_args=(
    --mode images
    --stack services
    --mcp-version "$MCP_VERSION"
    --gateway-version "$GATEWAY_VERSION"
  )
  if $DRY_RUN; then
    image_args+=(--dry-run)
  fi
  echo "[release] Building service images"
  run "$REPO_ROOT/build.sh" "${image_args[@]}"
fi

cat <<EOF

[release] Post-build checklist
  1. Review CHANGELOGs:
     - locus-core-rs/CHANGELOG.md (${CORE_VERSION})
     - locus-sdk/CHANGELOG.md (${SDK_VERSION})
     - locus-surreal-adapter/CHANGELOG.md (${ADAPTER_VERSION})
     - locus-wasm/CHANGELOG.md (${WASM_VERSION})
     - locus-gateway/CHANGELOG.md (${GATEWAY_VERSION})
     - locus-mcp/CHANGELOG.md (${MCP_VERSION})
     - locus-cli/CHANGELOG.md (${CLI_VERSION})
  2. STTP spec: docs/sttp_typed_ir_language_spec.md (1.2.0-draft)
  3. Build browser WASM bindings:
     cd locus-web && npm ci && npm run build:wasm
  4. Publish crates (core first, then adapter, then sdk):
     ./locus-core-rs/publish-crates.sh --publish
     ./locus-surreal-adapter/publish-crates.sh --publish
     ./locus-sdk/publish-crates.sh --publish
  5. Tag and push:
     git tag locus-core-rs/v${CORE_VERSION}
     git tag locus-sdk/v${SDK_VERSION}
     git tag locus-surreal-adapter/v${ADAPTER_VERSION}
     git tag locus-wasm/v${WASM_VERSION}
     git tag locus-mcp/v${MCP_VERSION}
     git tag locus-gateway/v${GATEWAY_VERSION}
     git tag locus-cli/v${CLI_VERSION}
     git push origin \\
       locus-core-rs/v${CORE_VERSION} \\
       locus-sdk/v${SDK_VERSION} \\
       locus-surreal-adapter/v${ADAPTER_VERSION} \\
       locus-wasm/v${WASM_VERSION} \\
       locus-mcp/v${MCP_VERSION} \\
       locus-gateway/v${GATEWAY_VERSION} \\
       locus-cli/v${CLI_VERSION}
  6. Push images (if built):
     docker push ghcr.io/entasislabs/locus-mcp:${MCP_VERSION}
     docker push ghcr.io/entasislabs/locus-gateway:${GATEWAY_VERSION}

EOF
