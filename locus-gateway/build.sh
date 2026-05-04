#!/usr/bin/env bash
# Build locus-gateway release artifacts for multiple targets.
#
# Usage:
#   ./build.sh [--publish] [--features <csv>] [--name-suffix <suffix>]
#
# Environment:
#   LOCUS_GATEWAY_VERSION   Artifact version (default: Cargo.toml version)
#   LOCUS_GATEWAY_RS_VERSION Fallback version key
#   LOCUS_GATEWAY_BUILD_FEATURES Optional cargo feature set
#   LOCUS_GATEWAY_BUILD_NAME_SUFFIX Optional artifact/release suffix (for example: embeddings)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_PATH="$SCRIPT_DIR/Cargo.toml"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

# Override order: LOCUS_GATEWAY_VERSION -> LOCUS_GATEWAY_RS_VERSION -> Cargo.toml version -> 0.1.0
CARGO_VERSION="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$MANIFEST_PATH" | head -n1 || true)"
VERSION="${LOCUS_GATEWAY_VERSION:-${LOCUS_GATEWAY_RS_VERSION:-${CARGO_VERSION:-0.1.0}}}"

BASE_NAME="locus-gateway"
BINARY_NAME="locus-gateway"
BUILD_FEATURES="${LOCUS_GATEWAY_BUILD_FEATURES:-}"
NAME_SUFFIX="${LOCUS_GATEWAY_BUILD_NAME_SUFFIX:-}"

TARGETS=(
  aarch64-apple-darwin
  x86_64-apple-darwin
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  x86_64-unknown-linux-musl
  x86_64-pc-windows-gnu
  aarch64-pc-windows-gnullvm
)

PUBLISH=false

usage() {
  cat <<'EOF'
Usage:
  ./build.sh [--publish] [--features <csv>] [--name-suffix <suffix>]

Options:
  --publish                 Upload packaged artifacts to GitHub release
  --features <csv>          Cargo features to build (example: local-embedding)
  --name-suffix <suffix>    Suffix artifact/release names (example: embeddings)
  -h, --help                Show help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --publish)
      PUBLISH=true
      shift
      ;;
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
      echo "error: unknown option '$1'" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -n "$NAME_SUFFIX" ]]; then
  NAME_SUFFIX="${NAME_SUFFIX#-}"
fi

NAME="$BASE_NAME"
if [[ -n "$NAME_SUFFIX" ]]; then
  NAME="${BASE_NAME}-${NAME_SUFFIX}"
fi

TAG_PREFIX="$NAME"
RELEASE="${TAG_PREFIX}/v${VERSION}"

BUILT_TARGETS=()

ensure_target() {
  local target="$1"
  if command -v rustup >/dev/null 2>&1; then
    rustup target add "$target" >/dev/null 2>&1 || true
  fi
}

run_build() {
  local target="$1"
  echo "[BUILD] cargo build --release --target $target ..."
  ensure_target "$target"

  local build_cmd=(
    cargo build
    --release
    --locked
    --manifest-path "$MANIFEST_PATH"
    --target "$target"
  )

  if [[ -n "$BUILD_FEATURES" ]]; then
    build_cmd+=(--features "$BUILD_FEATURES")
  fi

  if "${build_cmd[@]}"; then
    BUILT_TARGETS+=("$target")
  else
    echo "[WARN] Build failed for $target, skipping."
  fi
}

run_all_builds() {
  echo "[BUILD] Building for all targets..."
  for target in "${TARGETS[@]}"; do
    run_build "$target"
  done
}

package_artifact() {
  local target="$1"
  local bin_name="$BINARY_NAME"
  local archive_name=""

  case "$target" in
    aarch64-apple-darwin)
      archive_name="${NAME}-${VERSION}-macos-arm64.tar.gz"
      ;;
    x86_64-apple-darwin)
      archive_name="${NAME}-${VERSION}-macos-x64.tar.gz"
      ;;
    x86_64-unknown-linux-gnu)
      archive_name="${NAME}-${VERSION}-linux-x64.tar.gz"
      ;;
    aarch64-unknown-linux-gnu)
      archive_name="${NAME}-${VERSION}-linux-arm64.tar.gz"
      ;;
    x86_64-unknown-linux-musl)
      archive_name="${NAME}-${VERSION}-linux-musl-x64.tar.gz"
      ;;
    x86_64-pc-windows-gnu)
      archive_name="${NAME}-${VERSION}-win-x64.tar.gz"
      bin_name="${BINARY_NAME}.exe"
      ;;
    aarch64-pc-windows-gnullvm)
      archive_name="${NAME}-${VERSION}-win-arm64.tar.gz"
      bin_name="${BINARY_NAME}.exe"
      ;;
    *)
      echo "[WARN] Unknown target '$target', skipping packaging."
      return
      ;;
  esac

  local bin_path="$SCRIPT_DIR/target/$target/release/$bin_name"
  if [[ ! -f "$bin_path" ]]; then
    echo "[WARN] Missing binary for $target at $bin_path, skipping packaging."
    return
  fi

  tar -czf "$SCRIPT_DIR/$archive_name" -C "$(dirname "$bin_path")" "$bin_name"
  echo "  [OK] $archive_name"
}

package_all() {
  echo "[PACKAGE] Packaging artifacts..."
  for target in "${BUILT_TARGETS[@]}"; do
    package_artifact "$target"
  done
}

upload_all() {
  if ! $PUBLISH; then
    echo "[INFO] Skipping GitHub upload. Run with --publish to upload."
    return
  fi

  if ! command -v gh >/dev/null 2>&1; then
    echo "[ERROR] GitHub CLI (gh) not found. Install it: https://cli.github.com/"
    exit 1
  fi

  echo "[GITHUB] Uploading artifacts to $RELEASE..."

  if ! gh release view "$RELEASE" >/dev/null 2>&1; then
    echo "[GITHUB] Release $RELEASE does not exist. Creating..."
    gh release create "$RELEASE" --title "$RELEASE" --notes "Release $RELEASE"
  fi

  UPLOADS=()
  for target in "${BUILT_TARGETS[@]}"; do
    case "$target" in
      aarch64-apple-darwin) UPLOADS+=("${NAME}-${VERSION}-macos-arm64.tar.gz") ;;
      x86_64-apple-darwin) UPLOADS+=("${NAME}-${VERSION}-macos-x64.tar.gz") ;;
      x86_64-unknown-linux-gnu) UPLOADS+=("${NAME}-${VERSION}-linux-x64.tar.gz") ;;
      aarch64-unknown-linux-gnu) UPLOADS+=("${NAME}-${VERSION}-linux-arm64.tar.gz") ;;
      x86_64-unknown-linux-musl) UPLOADS+=("${NAME}-${VERSION}-linux-musl-x64.tar.gz") ;;
      x86_64-pc-windows-gnu) UPLOADS+=("${NAME}-${VERSION}-win-x64.tar.gz") ;;
      aarch64-pc-windows-gnullvm) UPLOADS+=("${NAME}-${VERSION}-win-arm64.tar.gz") ;;
    esac
  done

  if [[ ${#UPLOADS[@]} -eq 0 ]]; then
    echo "[GITHUB] No artifacts to upload."
    return
  fi

  local abs_uploads=()
  for item in "${UPLOADS[@]}"; do
    abs_uploads+=("$SCRIPT_DIR/$item")
  done

  gh release upload "$RELEASE" "${abs_uploads[@]}" --clobber
  echo "[GITHUB] Upload complete."
}

run_all_builds
package_all
upload_all
