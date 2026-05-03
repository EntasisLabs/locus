#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HTTP_PORT="${HTTP_PORT:-18080}"
GRPC_PORT="${GRPC_PORT:-18081}"
BASE_URL="http://127.0.0.1:${HTTP_PORT}"
LOG_FILE="${LOG_FILE:-/tmp/sttp-gateway-integration.log}"
EXTERNAL_GATEWAY="${EXTERNAL_GATEWAY:-0}"

GATEWAY_PID=""

cleanup() {
  if [[ -n "${GATEWAY_PID}" ]]; then
    kill "${GATEWAY_PID}" >/dev/null 2>&1 || true
    wait "${GATEWAY_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

fail() {
  echo "[FAIL] $1" >&2
  if [[ -f "${LOG_FILE}" ]]; then
    echo "----- gateway log tail -----" >&2
    tail -n 40 "${LOG_FILE}" >&2 || true
  fi
  exit 1
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local message="$3"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    fail "${message}. Missing substring: ${needle}. Body: ${haystack}"
  fi
}

escape_json_string() {
  printf '%s' "$1" | sed ':a;N;$!ba;s/\\/\\\\/g;s/"/\\"/g;s/\n/\\n/g'
}

post_json() {
  local path="$1"
  local payload="$2"
  local expected_status="$3"

  local response
  response="$(curl -sS -X POST "${BASE_URL}${path}" -H "content-type: application/json" -d "${payload}" -w $'\n%{http_code}')"
  local body="${response%$'\n'*}"
  local status="${response##*$'\n'}"

  if [[ "${status}" != "${expected_status}" ]]; then
    fail "POST ${path} expected HTTP ${expected_status}, got ${status}. Body: ${body}"
  fi

  printf "%s" "${body}"
}

if [[ "${EXTERNAL_GATEWAY}" != "1" ]]; then
  echo "[INFO] Starting locus-gateway on HTTP ${HTTP_PORT}, gRPC ${GRPC_PORT}"
  (
    cd "${ROOT_DIR}"
    cargo run --quiet -- \
      --http-port "${HTTP_PORT}" \
      --grpc-port "${GRPC_PORT}" \
      --backend in-memory \
      --cors-enabled false
  ) >"${LOG_FILE}" 2>&1 &
  GATEWAY_PID="$!"
else
  echo "[INFO] Using externally managed gateway at ${BASE_URL}"
fi

ready=0
for _ in $(seq 1 120); do
  if curl -fsS "${BASE_URL}/health" >/dev/null 2>&1; then
    ready=1
    break
  fi
  read -r -t 0.25 _ || true
done

if [[ "${ready}" != "1" ]]; then
  fail "Gateway did not become ready at ${BASE_URL}/health"
fi

echo "[INFO] Health check passed"

NODE_A='⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "session-a", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70 }, context_summary: "integration smoke A", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:30:00Z", tier: raw, session_id: "session-a", user_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 }, model_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩
◈⟨ { note(.99): "gateway integration smoke A" } ⟩
⍉⟨ { rho: 0.96, kappa: 0.94, psi: 2.60, compression_avec: { stability: 0.85, friction: 0.25, logic: 0.80, autonomy: 0.70, psi: 2.60 } } ⟩'

NODE_B='⊕⟨ { trigger: manual, response_format: temporal_node, origin_session: "session-b", compression_depth: 1, parent_node: null, prime: { attractor_config: { stability: 0.70, friction: 0.35, logic: 0.75, autonomy: 0.80 }, context_summary: "integration smoke B", relevant_tier: raw, retrieval_budget: 3 } } ⟩
⦿⟨ { timestamp: "2026-03-05T06:35:00Z", tier: raw, session_id: "session-b", user_avec: { stability: 0.70, friction: 0.35, logic: 0.75, autonomy: 0.80, psi: 2.60 }, model_avec: { stability: 0.70, friction: 0.35, logic: 0.75, autonomy: 0.80, psi: 2.60 } } ⟩
◈⟨ { note(.99): "gateway integration smoke B" } ⟩
⍉⟨ { rho: 0.95, kappa: 0.93, psi: 2.60, compression_avec: { stability: 0.70, friction: 0.35, logic: 0.75, autonomy: 0.80, psi: 2.60 } } ⟩'

store_a_payload="$(cat <<JSON
{"node":"$(escape_json_string "${NODE_A}")","sessionId":"session-a"}
JSON
)"
store_a_body="$(post_json "/api/v1/store" "${store_a_payload}" "200")"
assert_contains "${store_a_body}" '"valid":true' "session-a store response should be valid"

echo "[INFO] Stored session-a node"

store_b_payload="$(cat <<JSON
{"node":"$(escape_json_string "${NODE_B}")","sessionId":"session-b"}
JSON
)"
store_b_body="$(post_json "/api/v1/store" "${store_b_payload}" "200")"
assert_contains "${store_b_body}" '"valid":true' "session-b store response should be valid"

echo "[INFO] Stored session-b node"

scoped_context_payload='{"sessionId":"session-a","stability":0.85,"friction":0.25,"logic":0.80,"autonomy":0.70,"limit":10}'
scoped_context_body="$(post_json "/api/v1/context" "${scoped_context_payload}" "200")"
assert_contains "${scoped_context_body}" '"sessionId":"session-a"' "scoped context should include session-a"

if [[ "${scoped_context_body}" == *'"sessionId":"session-b"'* ]]; then
  fail "scoped context for session-a unexpectedly included session-b nodes"
fi

echo "[INFO] Scoped context check passed"

embedding_payload='{"sessionId":"session-a","stability":0.85,"friction":0.25,"logic":0.80,"autonomy":0.70,"limit":5,"ragEmbedding":[0.1,0.2,0.3],"avecEmbedding":[0.2,0.2,0.2],"ragWeight":0.7,"avecWeight":0.3,"alpha":0.65,"beta":0.35}'
embedding_body="$(post_json "/api/v1/context/embeddings" "${embedding_payload}" "200")"
assert_contains "${embedding_body}" '"retrieved":' "embedding retrieval response should include retrieved field"
assert_contains "${embedding_body}" '"sessionId":"session-a"' "embedding retrieval should return session-a for scoped query"

echo "[INFO] Embedding retrieval check passed"

invalid_dims_payload='{"sessionId":"session-a","stability":0.85,"friction":0.25,"logic":0.80,"autonomy":0.70,"ragEmbedding":[0.1,0.2,0.3],"avecEmbedding":[0.2,0.2]}'
invalid_dims_body="$(post_json "/api/v1/context/embeddings" "${invalid_dims_payload}" "400")"
assert_contains "${invalid_dims_body}" 'same dimensions' "invalid dimensions should return a validation error"

echo "[INFO] Dimension mismatch validation check passed"
echo "[PASS] Integration smoke checks completed successfully"
