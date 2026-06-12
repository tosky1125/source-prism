#!/usr/bin/env bash
set -euo pipefail

api_base_url="${API_BASE_URL:-http://127.0.0.1:4096}"
api_log="${API_LOG:-/tmp/source-prism-api.log}"
repo_id="${SOURCE_PRISM_CI_REPO_ID:-source-prism-ci}"

api_pid=""

cleanup() {
  local status=$?
  if [ "$status" -ne 0 ]; then
    echo "api smoke failed with exit status ${status}" >&2
    if [ -f /tmp/source-prism-api-last-response.txt ]; then
      cat /tmp/source-prism-api-last-response.txt >&2
    fi
    if [ -f "$api_log" ]; then
      tail -200 "$api_log" >&2
    fi
  fi
  if [ -n "$api_pid" ]; then
    kill "$api_pid" 2>/dev/null || true
    wait "$api_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

cargo run -p ri-api > "$api_log" 2>&1 &
api_pid=$!

wait_for_health() {
  for attempt in $(seq 1 30); do
    if curl -fsS "${api_base_url}/v1/health" > /tmp/source-prism-api-health.json 2>/dev/null &&
      grep -q '"status":"ok"' /tmp/source-prism-api-health.json
    then
      return 0
    fi
    sleep 1
  done

  echo "api health did not become ready: ${api_base_url}/v1/health" \
    > /tmp/source-prism-api-last-response.txt
  return 1
}

request() {
  local method="$1"
  local url="$2"
  local output="$3"
  shift 3

  local status
  status=$(curl -sS -o "$output" -w '%{http_code}' -X "$method" "$url" "$@") || {
    {
      echo "curl failed"
      echo "method=${method}"
      echo "url=${url}"
      echo "status=${status}"
      cat "$output" 2>/dev/null || true
    } > /tmp/source-prism-api-last-response.txt
    return 1
  }

  case "$status" in
    200 | 201)
      return 0
      ;;
    *)
      {
        echo "unexpected HTTP status"
        echo "method=${method}"
        echo "url=${url}"
        echo "status=${status}"
        cat "$output" 2>/dev/null || true
      } > /tmp/source-prism-api-last-response.txt
      return 1
      ;;
  esac
}

wait_for_health

request POST "${api_base_url}/v1/repos" /tmp/source-prism-api-repo.json \
  -H 'content-type: application/json' \
  --data "{\"repo_id\":\"${repo_id}\",\"name\":\"${repo_id}\",\"default_branch\":\"main\"}"
grep -q '"kind":"repo"' /tmp/source-prism-api-repo.json
grep -q '"persisted":true' /tmp/source-prism-api-repo.json

request POST "${api_base_url}/v1/repos/${repo_id}/index" /tmp/source-prism-api-index.json \
  -H 'content-type: application/json' \
  --data '{"sha":"HEAD","repo_path":"."}'
grep -q '"kind":"index"' /tmp/source-prism-api-index.json
grep -q '"status":"succeeded"' /tmp/source-prism-api-index.json
grep -q '"inserted_file_manifests":' /tmp/source-prism-api-index.json
grep -q '"indexed_symbols":' /tmp/source-prism-api-index.json
grep -q '"indexed_graph_edges":' /tmp/source-prism-api-index.json
grep -q '"indexed_import_edges":' /tmp/source-prism-api-index.json
grep -q '"indexed_call_edges":' /tmp/source-prism-api-index.json
grep -q '"indexed_test_cover_edges":' /tmp/source-prism-api-index.json
grep -q '"indexed_search_chunks":' /tmp/source-prism-api-index.json
grep -q '"indexed_test_cases":' /tmp/source-prism-api-index.json

run_id=$(python3 -c 'import json; print(json.load(open("/tmp/source-prism-api-index.json"))["run_id"])')
request GET "${api_base_url}/v1/runs/${run_id}" /tmp/source-prism-api-run.json
grep -q '"kind":"run"' /tmp/source-prism-api-run.json
grep -q '"status":"succeeded"' /tmp/source-prism-api-run.json

request POST "${api_base_url}/v1/context/search" /tmp/source-prism-api-context.json \
  -H 'content-type: application/json' \
  --data "{\"repo_id\":\"${repo_id}\",\"query\":\"search_context\"}"
grep -q '"kind":"context_search"' /tmp/source-prism-api-context.json
grep -q '"vector_only":false' /tmp/source-prism-api-context.json

request GET "${api_base_url}/v1/repos/${repo_id}/symbols" /tmp/source-prism-api-symbols.json
grep -q '"kind":"symbols"' /tmp/source-prism-api-symbols.json
grep -q '"symbol_count":' /tmp/source-prism-api-symbols.json

api_impact_symbol=$(
  python3 -c 'import json; data=json.load(open("/tmp/source-prism-api-symbols.json")); print(next(symbol["fqn"] for symbol in data["symbols"] if symbol["kind"] == "function"))'
)
request POST "${api_base_url}/v1/impact" /tmp/source-prism-api-impact.json \
  -H 'content-type: application/json' \
  --data "{\"repo_id\":\"${repo_id}\",\"symbol\":\"${api_impact_symbol}\"}"
grep -q '"kind":"impact"' /tmp/source-prism-api-impact.json
grep -q '"impact_score":' /tmp/source-prism-api-impact.json

request GET "${api_base_url}/repo/${repo_id}" /tmp/source-prism-api-web.html
grep -q 'Repo Structure Explorer' /tmp/source-prism-api-web.html
grep -q "data-repo-id=\"${repo_id}\"" /tmp/source-prism-api-web.html
grep -q 'References' /tmp/source-prism-api-web.html
grep -q 'api("references")' /tmp/source-prism-api-web.html
grep -q 'Coverage Evidence' /tmp/source-prism-api-web.html
grep -q 'api("coverage")' /tmp/source-prism-api-web.html

request GET "${api_base_url}/v1/repos/${repo_id}/tests" /tmp/source-prism-api-tests.json
grep -q '"kind":"tests"' /tmp/source-prism-api-tests.json
grep -q '"test_count":' /tmp/source-prism-api-tests.json

request GET "${api_base_url}/v1/repos/${repo_id}/references?symbol=${api_impact_symbol}" \
  /tmp/source-prism-api-references.json
grep -q '"kind":"references"' /tmp/source-prism-api-references.json
grep -q '"incoming_count":' /tmp/source-prism-api-references.json
grep -q '"outgoing_count":' /tmp/source-prism-api-references.json

request GET "${api_base_url}/v1/repos/${repo_id}/coverage" /tmp/source-prism-api-coverage.json
grep -q '"kind":"coverage"' /tmp/source-prism-api-coverage.json
grep -q '"segment_count":' /tmp/source-prism-api-coverage.json

api_test_symbol=$(
  python3 -c 'import json; data=json.load(open("/tmp/source-prism-api-symbols.json")); print(next(symbol["fqn"] for symbol in data["symbols"] if symbol["kind"] == "test_case"))'
)
request POST "${api_base_url}/v1/test-context" /tmp/source-prism-api-test-context.json \
  -H 'content-type: application/json' \
  --data "{\"repo_id\":\"${repo_id}\",\"symbol\":\"${api_test_symbol}\"}"
grep -q '"kind":"test_context"' /tmp/source-prism-api-test-context.json
grep -q '"code_execution_allowed":false' /tmp/source-prism-api-test-context.json

request GET "${api_base_url}/v1/repos/${repo_id}/graph" /tmp/source-prism-api-graph.json
grep -q '"kind":"graph"' /tmp/source-prism-api-graph.json
grep -q '"edge_count":' /tmp/source-prism-api-graph.json
grep -q '"edge_type":"imports"' /tmp/source-prism-api-graph.json
grep -q '"edge_type":"calls"' /tmp/source-prism-api-graph.json
grep -q '"edge_type":"test_covers"' /tmp/source-prism-api-graph.json

request GET "${api_base_url}/v1/repos/local/files" /tmp/source-prism-api-files.json
grep -q '"kind":"files"' /tmp/source-prism-api-files.json
grep -q '"file_count":' /tmp/source-prism-api-files.json
