#!/usr/bin/env bash
# Build locus-cli release artifacts for multiple targets.
#
# Usage:
#   ./build.sh [--publish]
#
# Environment:
#   LOCUS_CLI_VERSION   Artifact version (default: Cargo.toml version)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_PATH="$SCRIPT_DIR/Cargo.toml"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi

# Override order: LOCUS_CLI_VERSION -> Cargo.toml version -> 0.1.0
CARGO_VERSION="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$MANIFEST_PATH" | head -n1 || true)"
VERSION="${LOCUS_CLI_VERSION:-${CARGO_VERSION:-0.1.0}}"

TAG_PREFIX="locus-cli"
RELEASE="${TAG_PREFIX}/v${VERSION}"
NAME="locus-cli"

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
if [[ "${1:-}" == "--publish" ]]; then
  PUBLISH=true
fi

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
  if cargo build --release --locked --manifest-path "$MANIFEST_PATH" --target "$target"; then
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
  local bin_name="$NAME"
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
      bin_name="${NAME}.exe"
      ;;
    aarch64-pc-windows-gnullvm)
      archive_name="${NAME}-${VERSION}-win-arm64.tar.gz"
      bin_name="${NAME}.exe"
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
