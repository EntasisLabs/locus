#!/usr/bin/env bash
# Preflight and optional publish helper for locus-core-rs.
#
# Usage:
#   ./publish-crates.sh            # run checks + dry-run publish
#   ./publish-crates.sh --publish  # run checks + dry-run + real publish
#
# Environment:
#   LOCUS_SKIP_TESTS=1   Skip cargo test in preflight.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_PATH="$SCRIPT_DIR/Cargo.toml"
DO_PUBLISH="${1:-}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

echo "Running preflight: cargo check"
cargo check --manifest-path "$MANIFEST_PATH"

if [[ "${LOCUS_SKIP_TESTS:-0}" != "1" ]]; then
  echo "Running preflight: cargo test"
  cargo test --manifest-path "$MANIFEST_PATH"
fi

echo "Reviewing package contents"
cargo package --manifest-path "$MANIFEST_PATH" --list

echo "Running publish dry-run"
cargo publish --manifest-path "$MANIFEST_PATH" --dry-run

if [[ "$DO_PUBLISH" == "--publish" ]]; then
  echo "Publishing crate"
  cargo publish --manifest-path "$MANIFEST_PATH"
else
  echo ""
  echo "Dry-run complete."
  echo "To publish for real, run:"
  echo "  ./locus-core-rs/publish-crates.sh --publish"
fi
