#!/usr/bin/env bash
# Locus master build wrapper.
#
# Orchestrates existing per-component release/image scripts while allowing
# per-component version overrides.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

MODE="all"
STACK="all"
TARGETS_RAW=""

DEFAULT_VERSION=""
MCP_VERSION=""
GATEWAY_VERSION=""
CLI_VERSION=""

DEFAULT_FEATURES=""
MCP_FEATURES=""
GATEWAY_FEATURES=""
CLI_FEATURES=""

DEFAULT_NAME_SUFFIX=""
MCP_NAME_SUFFIX=""
GATEWAY_NAME_SUFFIX=""
CLI_NAME_SUFFIX=""

IMAGE_PREFIX="ghcr.io/entasislabs"
LOCAL_IMAGE_TAGS=false
PUBLISH=false
DRY_RUN=false

usage() {
  cat <<'EOF'
Usage:
  ./build.sh [options]

Modes:
  --mode release|images|all   Build release archives/checks, Docker images, or both (default: all)

Targeting:
  --stack all|hosts|services|core   Default target set (default: all)
  --targets a,b,c                    Explicit targets (overrides --stack)

Valid targets:
  core, mcp, gateway, cli

Version controls:
  --default-version X.Y.Z
  --mcp-version X.Y.Z
  --gateway-version X.Y.Z
  --cli-version X.Y.Z

Feature controls:
  --features csv
  --mcp-features csv
  --gateway-features csv
  --cli-features csv

Name controls:
  --name-suffix suffix
  --mcp-name-suffix suffix
  --gateway-name-suffix suffix
  --cli-name-suffix suffix

Image controls:
  --image-prefix ghcr.io/entasislabs   Prefix for non-local image tags
  --local-image-tags                 Use local tags like locus-mcp:0.1.0

Other:
  --publish    Forward --publish into per-component release scripts
  --dry-run    Print commands without executing them
  -h, --help   Show help

Examples:
  # Full release + images using one shared version
  ./build.sh --default-version 0.1.0

  # Publish release archives/checks only
  ./build.sh --mode release --default-version 0.1.0 --publish

  # Build only service images (mcp+gateway) with local tags
  ./build.sh --mode images --stack services --default-version 0.1.0 --local-image-tags

  # Mixed versions for explicit targets
  ./build.sh --mode all --targets mcp,gateway,cli --mcp-version 0.1.0 --gateway-version 2.0.0 --cli-version 0.1.0

  # Build local-embedding variants tagged as *-embeddings
  ./build.sh --targets mcp,gateway --features local-embedding --name-suffix embeddings
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

run_in_dir() {
  local dir="$1"
  shift

  if $DRY_RUN; then
    printf '[DRY-RUN] (cd %s &&' "$dir"
    for arg in "$@"; do
      printf ' %q' "$arg"
    done
    echo ")"
    return 0
  fi

  (
    cd "$dir"
    "$@"
  )
}

add_target_if_missing() {
  local candidate="$1"
  local existing

  for existing in "${TARGETS[@]:-}"; do
    if [[ "$existing" == "$candidate" ]]; then
      return 0
    fi
  done

  TARGETS+=("$candidate")
}

expand_and_add_target() {
  local token="$1"
  case "$token" in
    all)
      add_target_if_missing "core"
      add_target_if_missing "mcp"
      add_target_if_missing "gateway"
      add_target_if_missing "cli"
      ;;
    hosts)
      add_target_if_missing "mcp"
      add_target_if_missing "gateway"
      add_target_if_missing "cli"
      ;;
    services)
      add_target_if_missing "mcp"
      add_target_if_missing "gateway"
      ;;
    core)
      add_target_if_missing "core"
      ;;
    mcp|gateway|cli)
      add_target_if_missing "$token"
      ;;
    *)
      die "Unknown target or stack token: $token"
      ;;
  esac
}

resolve_version_for_target() {
  local target="$1"
  case "$target" in
    mcp)
      echo "${MCP_VERSION:-${DEFAULT_VERSION:-0.1.0}}"
      ;;
    gateway)
      echo "${GATEWAY_VERSION:-${DEFAULT_VERSION:-2.0.0}}"
      ;;
    cli)
      echo "${CLI_VERSION:-${DEFAULT_VERSION:-0.1.0}}"
      ;;
    core)
      echo "n/a"
      ;;
    *)
      die "Unknown target in version resolver: $target"
      ;;
  esac
}

image_name_for_target() {
  local target="$1"
  local suffix="$2"
  local name=""
  case "$target" in
    mcp) name="locus-mcp" ;;
    gateway) name="locus-gateway" ;;
    *) die "Unknown image-capable target: $1" ;;
  esac

  if [[ -n "$suffix" ]]; then
    suffix="${suffix#-}"
    name="${name}-${suffix}"
  fi

  echo "$name"
}

build_image_tag() {
  local target="$1"
  local version="$2"
  local suffix="$3"
  local image_name
  image_name="$(image_name_for_target "$target" "$suffix")"

  if $LOCAL_IMAGE_TAGS; then
    echo "${image_name}:${version}"
    return 0
  fi

  local prefix="${IMAGE_PREFIX%/}"
  echo "${prefix}/${image_name}:${version}"
}

resolve_features_for_target() {
  case "$1" in
    mcp)
      echo "${MCP_FEATURES:-$DEFAULT_FEATURES}"
      ;;
    gateway)
      echo "${GATEWAY_FEATURES:-$DEFAULT_FEATURES}"
      ;;
    cli)
      echo "${CLI_FEATURES:-$DEFAULT_FEATURES}"
      ;;
    core)
      echo ""
      ;;
    *)
      die "Unknown target in feature resolver: $1"
      ;;
  esac
}

resolve_name_suffix_for_target() {
  case "$1" in
    mcp)
      echo "${MCP_NAME_SUFFIX:-$DEFAULT_NAME_SUFFIX}"
      ;;
    gateway)
      echo "${GATEWAY_NAME_SUFFIX:-$DEFAULT_NAME_SUFFIX}"
      ;;
    cli)
      echo "${CLI_NAME_SUFFIX:-$DEFAULT_NAME_SUFFIX}"
      ;;
    core)
      echo ""
      ;;
    *)
      die "Unknown target in name suffix resolver: $1"
      ;;
  esac
}

run_release_for_target() {
  local target="$1"
  local version="$2"
  local features
  local name_suffix
  local -a extra_args=()

  features="$(resolve_features_for_target "$target")"
  name_suffix="$(resolve_name_suffix_for_target "$target")"

  if [[ -n "$features" ]]; then
    extra_args+=(--features "$features")
  fi
  if [[ -n "$name_suffix" ]]; then
    extra_args+=(--name-suffix "$name_suffix")
  fi
  if $PUBLISH; then
    extra_args+=(--publish)
  fi

  case "$target" in
    core)
      if $PUBLISH; then
        run_in_dir "$SCRIPT_DIR/locus-core" bash ./publish-crates.sh --publish
      else
        run_in_dir "$SCRIPT_DIR/locus-core" bash ./publish-crates.sh
      fi
      ;;
    mcp)
      run_in_dir "$SCRIPT_DIR/locus-mcp" env LOCUS_MCP_VERSION="$version" LOCUS_VERSION="$version" bash ./build.sh "${extra_args[@]}"
      ;;
    gateway)
      run_in_dir "$SCRIPT_DIR/locus-gateway" env LOCUS_GATEWAY_VERSION="$version" LOCUS_GATEWAY_RS_VERSION="$version" bash ./build.sh "${extra_args[@]}"
      ;;
    cli)
      run_in_dir "$SCRIPT_DIR/locus-cli" env LOCUS_CLI_VERSION="$version" bash ./build.sh "${extra_args[@]}"
      ;;
    *)
      die "Unknown target in release mode: $target"
      ;;
  esac
}

run_images_for_target() {
  local target="$1"
  local version="$2"
  local features
  local name_suffix

  features="$(resolve_features_for_target "$target")"
  name_suffix="$(resolve_name_suffix_for_target "$target")"

  case "$target" in
    mcp)
      if [[ -n "$features" ]]; then
        run_in_dir "$SCRIPT_DIR/locus-mcp" bash ./build-image.sh "$(build_image_tag "$target" "$version" "$name_suffix")" --features "$features" --name-suffix "$name_suffix"
      else
        run_in_dir "$SCRIPT_DIR/locus-mcp" bash ./build-image.sh "$(build_image_tag "$target" "$version" "$name_suffix")" --name-suffix "$name_suffix"
      fi
      ;;
    gateway)
      if [[ -n "$features" ]]; then
        run_in_dir "$SCRIPT_DIR/locus-gateway" bash ./build-image.sh "$(build_image_tag "$target" "$version" "$name_suffix")" --features "$features" --name-suffix "$name_suffix"
      else
        run_in_dir "$SCRIPT_DIR/locus-gateway" bash ./build-image.sh "$(build_image_tag "$target" "$version" "$name_suffix")" --name-suffix "$name_suffix"
      fi
      ;;
    core|cli)
      echo "[INFO] Skipping image mode for $target: no build-image.sh is defined."
      ;;
    *)
      die "Unknown target in image mode: $target"
      ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      [[ $# -ge 2 ]] || die "Missing value for --mode"
      MODE="$2"
      shift 2
      ;;
    --stack)
      [[ $# -ge 2 ]] || die "Missing value for --stack"
      STACK="$2"
      shift 2
      ;;
    --targets)
      [[ $# -ge 2 ]] || die "Missing value for --targets"
      TARGETS_RAW="$2"
      shift 2
      ;;
    --default-version)
      [[ $# -ge 2 ]] || die "Missing value for --default-version"
      DEFAULT_VERSION="$2"
      shift 2
      ;;
    --mcp-version)
      [[ $# -ge 2 ]] || die "Missing value for --mcp-version"
      MCP_VERSION="$2"
      shift 2
      ;;
    --gateway-version)
      [[ $# -ge 2 ]] || die "Missing value for --gateway-version"
      GATEWAY_VERSION="$2"
      shift 2
      ;;
    --cli-version)
      [[ $# -ge 2 ]] || die "Missing value for --cli-version"
      CLI_VERSION="$2"
      shift 2
      ;;
    --features)
      [[ $# -ge 2 ]] || die "Missing value for --features"
      DEFAULT_FEATURES="$2"
      shift 2
      ;;
    --mcp-features)
      [[ $# -ge 2 ]] || die "Missing value for --mcp-features"
      MCP_FEATURES="$2"
      shift 2
      ;;
    --gateway-features)
      [[ $# -ge 2 ]] || die "Missing value for --gateway-features"
      GATEWAY_FEATURES="$2"
      shift 2
      ;;
    --cli-features)
      [[ $# -ge 2 ]] || die "Missing value for --cli-features"
      CLI_FEATURES="$2"
      shift 2
      ;;
    --name-suffix)
      [[ $# -ge 2 ]] || die "Missing value for --name-suffix"
      DEFAULT_NAME_SUFFIX="$2"
      shift 2
      ;;
    --mcp-name-suffix)
      [[ $# -ge 2 ]] || die "Missing value for --mcp-name-suffix"
      MCP_NAME_SUFFIX="$2"
      shift 2
      ;;
    --gateway-name-suffix)
      [[ $# -ge 2 ]] || die "Missing value for --gateway-name-suffix"
      GATEWAY_NAME_SUFFIX="$2"
      shift 2
      ;;
    --cli-name-suffix)
      [[ $# -ge 2 ]] || die "Missing value for --cli-name-suffix"
      CLI_NAME_SUFFIX="$2"
      shift 2
      ;;
    --image-prefix)
      [[ $# -ge 2 ]] || die "Missing value for --image-prefix"
      IMAGE_PREFIX="$2"
      shift 2
      ;;
    --local-image-tags)
      LOCAL_IMAGE_TAGS=true
      shift
      ;;
    --publish)
      PUBLISH=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "Unknown option: $1"
      ;;
  esac
done

case "$MODE" in
  release|images|all)
    ;;
  *)
    die "Invalid --mode '$MODE' (expected: release, images, all)"
    ;;
esac

case "$STACK" in
  all|hosts|services|core)
    ;;
  *)
    die "Invalid --stack '$STACK' (expected: all, hosts, services, core)"
    ;;
esac

declare -a TARGETS=()

if [[ -n "$TARGETS_RAW" ]]; then
  IFS=',' read -r -a target_tokens <<< "$TARGETS_RAW"
  for token in "${target_tokens[@]}"; do
    normalized="$(echo "$token" | tr -d '[:space:]')"
    [[ -n "$normalized" ]] || continue
    expand_and_add_target "$normalized"
  done
else
  expand_and_add_target "$STACK"
fi

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  die "No targets resolved."
fi

echo "[INFO] Mode: $MODE"
echo "[INFO] Targets: ${TARGETS[*]}"
for target in "${TARGETS[@]}"; do
  version="$(resolve_version_for_target "$target")"
  echo "[INFO] - $target => version $version"
  features="$(resolve_features_for_target "$target")"
  suffix="$(resolve_name_suffix_for_target "$target")"
  if [[ -n "$features" ]]; then
    echo "[INFO]   features: $features"
  fi
  if [[ -n "$suffix" ]]; then
    echo "[INFO]   name-suffix: ${suffix#-}"
  fi
done

if [[ "$MODE" == "release" || "$MODE" == "all" ]]; then
  echo "[INFO] Running release builds..."
  for target in "${TARGETS[@]}"; do
    version="$(resolve_version_for_target "$target")"
    run_release_for_target "$target" "$version"
  done
fi

if [[ "$MODE" == "images" || "$MODE" == "all" ]]; then
  echo "[INFO] Running image builds..."
  for target in "${TARGETS[@]}"; do
    version="$(resolve_version_for_target "$target")"
    run_images_for_target "$target" "$version"
  done
fi

echo "[INFO] Done."
