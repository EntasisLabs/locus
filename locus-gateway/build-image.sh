#!/usr/bin/env bash
# Build (and optionally push) the locus-gateway Docker image.
#
# Usage:
#   ./build-image.sh [IMAGE_TAG] [FEATURES]
#   ./build-image.sh [--features <csv>] [--name-suffix <suffix>] [IMAGE_TAG]
#
# Default IMAGE_TAG: ghcr.io/keryxlabs/locus-gateway:2.0.0
# Optional features: set LOCUS_GATEWAY_BUILD_FEATURES (for example: local-embedding)
# Optional name suffix: set LOCUS_GATEWAY_BUILD_NAME_SUFFIX (for example: embeddings)
#
# Builds the Rust binary on the host first, then packages publish output into
# a minimal runtime image. No Rust toolchain is required inside the container.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_FEATURES="${LOCUS_GATEWAY_BUILD_FEATURES:-}"
NAME_SUFFIX="${LOCUS_GATEWAY_BUILD_NAME_SUFFIX:-}"
BASE_IMAGE_NAME="locus-gateway"
DEFAULT_VERSION="2.0.0"
PUBLISH_DIR="$SCRIPT_DIR/publish"

usage() {
  cat <<'EOF'
Usage:
  ./build-image.sh [IMAGE_TAG] [FEATURES]
  ./build-image.sh [--features <csv>] [--name-suffix <suffix>] [IMAGE_TAG]

Options:
  --features <csv>          Cargo features to build (example: local-embedding)
  --name-suffix <suffix>    Suffix default image name (example: embeddings)
  -h, --help                Show help
EOF
}

POSITIONAL=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --features)
      [[ $# -ge 2 ]] || { echo "error: missing value for --features" >&2; exit 1; }
      BUILD_FEATURES="$2"
      shift 2
      ;;
    --name-suffix)
      [[ $# -ge 2 ]] || { echo "error: missing value for --name-suffix" >&2; exit 1; }
      NAME_SUFFIX="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

if [[ -n "$NAME_SUFFIX" ]]; then
  NAME_SUFFIX="${NAME_SUFFIX#-}"
fi

IMAGE_NAME="$BASE_IMAGE_NAME"
if [[ -n "$NAME_SUFFIX" ]]; then
  IMAGE_NAME="${BASE_IMAGE_NAME}-${NAME_SUFFIX}"
fi

IMAGE_TAG="ghcr.io/keryxlabs/${IMAGE_NAME}:${DEFAULT_VERSION}"
if [[ ${#POSITIONAL[@]} -ge 1 ]]; then
  IMAGE_TAG="${POSITIONAL[0]}"
fi
if [[ ${#POSITIONAL[@]} -ge 2 && -z "$BUILD_FEATURES" ]]; then
  BUILD_FEATURES="${POSITIONAL[1]}"
fi
if [[ ${#POSITIONAL[@]} -gt 2 ]]; then
  echo "error: too many positional arguments" >&2
  usage >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not found. Install Docker first." >&2
  exit 1
fi

echo "Publishing binary on host..."
build_cmd=(
  cargo build
  --release
  --locked
  --manifest-path "$SCRIPT_DIR/Cargo.toml"
)

if [[ -n "$BUILD_FEATURES" ]]; then
  echo "Enabling cargo features: $BUILD_FEATURES"
  build_cmd+=(--features "$BUILD_FEATURES")
fi

"${build_cmd[@]}"

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
