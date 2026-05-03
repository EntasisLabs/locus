#!/usr/bin/env bash
# Build (and optionally push) the locus-gateway Docker image.
#
# Usage:
#   ./build-image.sh [IMAGE_TAG]
#
# Default IMAGE_TAG: ghcr.io/keryxlabs/locus-gateway:2.0.0
#
# Builds the Rust binary on the host first, then packages publish output into
# a minimal runtime image. No Rust toolchain is required inside the container.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_TAG="${1:-ghcr.io/keryxlabs/locus-gateway:2.0.0}"
PUBLISH_DIR="$SCRIPT_DIR/publish"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not found. Install Docker first." >&2
  exit 1
fi

echo "Publishing binary on host..."
cargo build \
  --release \
  --locked \
  --manifest-path "$SCRIPT_DIR/Cargo.toml"

mkdir -p "$PUBLISH_DIR"
cp "$SCRIPT_DIR/target/release/locus-gateway" "$PUBLISH_DIR/locus-gateway"
chmod +x "$PUBLISH_DIR/locus-gateway"

if command -v strip >/dev/null 2>&1; then
  strip "$PUBLISH_DIR/locus-gateway" || true
fi

echo "Building $IMAGE_TAG..."
docker build \
  -f "$SCRIPT_DIR/Dockerfile" \
  -t "$IMAGE_TAG" \
  "$REPO_ROOT"

echo ""
echo "Built:  $IMAGE_TAG"
echo "Push:   docker push $IMAGE_TAG"
