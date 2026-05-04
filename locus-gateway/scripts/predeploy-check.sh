#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WITH_DOCKER=0

usage() {
  cat <<'EOF'
Usage: ./scripts/predeploy-check.sh [--with-docker]

Runs predeploy validation for locus-gateway:
  1) cargo test --tests
  2) cargo test --tests --features local-embedding
  3) integration smoke (in-memory)
  4) optional dockerized smoke when --with-docker is provided
EOF
}

for arg in "$@"; do
  case "$arg" in
    --with-docker)
      WITH_DOCKER=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      usage
      exit 1
      ;;
  esac
done

run_step() {
  local label="$1"
  shift
  echo "[STEP] ${label}"
  "$@"
  echo "[OK] ${label}"
}

cleanup() {
  if [[ -n "${DOCKER_CID:-}" ]]; then
    docker rm -f "${DOCKER_CID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cd "${ROOT_DIR}"

run_step "Unit tests (default)" cargo test --tests
run_step "Unit tests (local-embedding)" cargo test --tests --features local-embedding
run_step "Integration smoke (local process)" ./scripts/integration-smoke.sh

if [[ "${WITH_DOCKER}" == "1" ]]; then
  run_step "Docker image build" docker build -t locus-gateway:predeploy .

  echo "[STEP] Start dockerized gateway"
  DOCKER_CID="$(docker run -d -p 28080:18080 -p 28081:18081 locus-gateway:predeploy --http-port 18080 --grpc-port 18081 --backend in-memory --cors-enabled false)"
  echo "[OK] Start dockerized gateway (${DOCKER_CID})"

  run_step "Integration smoke (dockerized gateway)" env EXTERNAL_GATEWAY=1 HTTP_PORT=28080 GRPC_PORT=28081 ./scripts/integration-smoke.sh
fi

echo "[PASS] Predeploy checks completed successfully"
