#!/usr/bin/env bash
# Build a local locus-core crate artifact.
#
# Usage:
#   ./build-package.sh
#
# Environment:
#   LOCUS_SKIP_TESTS=1   Skip `cargo test` before packaging.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_PATH="$SCRIPT_DIR/Cargo.toml"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

echo "Checking crate..."
cargo check --manifest-path "$MANIFEST_PATH"

if [[ "${LOCUS_SKIP_TESTS:-0}" != "1" ]]; then
  echo "Running tests..."
  cargo test --manifest-path "$MANIFEST_PATH"
fi

echo "Packaging crate artifact..."
cargo package --manifest-path "$MANIFEST_PATH" --allow-dirty

echo ""
echo "Built package artifacts:"
ls -1 "$SCRIPT_DIR/target/package"/*.crate
