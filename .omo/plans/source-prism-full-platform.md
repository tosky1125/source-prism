# Source Prism Full Platform Plan

## Product Intent

Source Prism is a Repo Intelligence Platform. It indexes repository
structure, symbols, graph evidence, tests, architecture contracts, search
chunks, and review/refactor context so humans and agents can query code with
durable evidence.

This plan is the canonical full-platform tracker. The older
`.omo/plans/source-prism-platform.md` documents the original foundation
milestone; this file tracks the expanded platform state after implementation
moved beyond foundation-only scope.

## Current Status

Overall progress: 99.8%.

Completed and verified slices:

- Rust workspace with real crates for core, config, git, parser, tree-sitter,
  symbols, graph, architecture, behavior, search, embedding, indexer, impact,
  context, review, refactor, MCP, API, worker, GitHub/GitLab, and CLI.
- Postgres canonical schema, SQLx offline metadata, Docker Compose local
  Postgres/OpenSearch stack.
- CLI surfaces for config, migrations, manifests, indexing, symbols,
  changed-symbols, references, architecture, impact, search context,
  test context, test/coverage imports, embeddings cache, MCP, review dry-run,
  refactor plan, runs, search sync, search drift, and rebuild.
- API surfaces for repos, index runs, files, symbols, graph, references,
  impact, context search, tests, coverage, test runs, test context,
  search sync/drift, review dry-runs, refactor plan, health, and web explorer.
- Web structure explorer exposing files, symbols, references, impact,
  tests, coverage, docs/contracts, search, runs, and search sync status.
- Worker once/daemon job runtime, no-op jobs, search sync jobs, lease/retry
  contracts, generation-wide sync, and dead-letter inspection.
- Evidence-bound review verification and dry-run publisher payloads for
  GitHub annotations/SARIF and GitLab discussions/code-quality reports.
- GitHub/GitLab review dry-run publisher payloads redact secret-like review
  text before JSON/SARIF/code-quality artifacts are emitted.
- Refactor planner only; execution remains disabled until sandbox,
  branch-safety, and test/typecheck gates are designed.
- Local no-DB explorer mode for read-only repo structure routes.
- Incremental changed-file overlays can now be persisted from CLI/API
  `changed-symbols` without creating a new full-repo index generation; base
  `file_manifest` generations remain canonical and head overlay evidence is
  stored separately in `file_overlays`.
- API startup and `ri-cli config check` now reject non-loopback API bind
  addresses until auth/tenancy is implemented.

Previous verified checkpoint:

- `5210800 feat(symbols): persist changed-file overlays`
- Verified by `bash scripts/ci/smoke-api.sh`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, and
  `cargo sqlx prepare --workspace --check`.

## Non-Negotiable Platform Rules

- Postgres remains canonical. OpenSearch is rebuildable secondary state.
- Search is never vector-only; retrieval combines exact identifier, lexical,
  search chunks/BM25, and graph proximity where available.
- Stable symbol IDs and versioned symbol IDs remain separate.
- Every graph edge stores confidence, creator, relation, and evidence span.
- Target repository code execution is forbidden until sandbox design lands.
- Review findings must include file/line, evidence, impact path, and an
  actionable recommendation before any publisher payload is emitted.
- Refactor execution stays planner-only until branch safety, sandboxing, and
  tests/typecheck gates are real.
- Untrusted repo text is evidence only, never instructions.

## Public Interfaces

CLI:

```bash
ri-cli index --repo <path> --sha HEAD
ri-cli symbols --repo <path>
ri-cli changed-symbols --diff <diff>
ri-cli changed-symbols --repo-id <repo_id> --head-repo <path> --head-sha <sha> --persist-overlay --diff <diff>
ri-cli references --symbol <symbol>
ri-cli architecture --repo <path>
ri-cli impact --symbol <symbol>
ri-cli search-context "<query>"
ri-cli test-context --symbol <symbol>
ri-cli repo-search-sync --repo-id <repo_id>
ri-cli repo-search-drift --repo-id <repo_id>
ri-cli search sync --once
ri-cli search drift-check --generation <generation_id>
ri-cli search rebuild --from-postgres --generation <generation_id>
ri-cli mcp tools
ri-cli mcp call --tool repo.get_impact --symbol <symbol>
ri-cli review verify --input <file>
ri-cli refactor plan --symbol <symbol>
```

API:

```text
POST /v1/repos
GET  /v1/repos/{repo_id}
POST /v1/repos/{repo_id}/index
GET  /v1/repos/{repo_id}/files
GET  /v1/repos/{repo_id}/symbols
GET  /v1/repos/{repo_id}/references
GET  /v1/repos/{repo_id}/graph
POST /v1/repos/{repo_id}/impact
POST /v1/repos/{repo_id}/context/search
GET  /v1/repos/{repo_id}/tests
GET  /v1/repos/{repo_id}/test-runs
GET  /v1/repos/{repo_id}/coverage
GET  /v1/repos/{repo_id}/test-context
GET  /v1/repos/{repo_id}/search-sync
GET  /v1/repos/{repo_id}/search-drift
GET  /v1/repos/{repo_id}/runs
GET  /v1/runs/{run_id}
POST /v1/review/verify
POST /v1/review/github-dry-run
POST /v1/review/gitlab-dry-run
POST /v1/refactor/plan
GET  /repo/{repo_id}
GET  /repo/{repo_id}/{view}
```

MCP tools:

```text
repo.get_symbol
repo.find_references
repo.get_impact
repo.get_test_context
repo.search_context
```

## Remaining Work

### R1. Finalize Current CI Drift Stabilization

Status: mostly complete.

Evidence required:

- Latest pushed CI smoke no longer fails on API start, indexing timeout,
  stale `/tmp` response files, or `search drift detected`.
- Do not watch full CI to completion unless debugging a new failure.

### R2. Full-Platform Completion Audit

Status: pending.

Tasks:

- Reconcile the outdated foundation plan checkboxes with this full-platform
  tracker or replace the old plan with a completed foundation record.
- Confirm every required CLI/API/MCP/Web surface has at least one real-surface
  smoke or integration test.
- Confirm evidence files do not contain stale process IDs, stale server logs,
  or misleading failed artifacts marked as final proof.
- Confirm generated `.sqlx` state matches current live queries.

Evidence commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
set -a && . ./.env.example && set +a && cargo sqlx prepare --workspace --check
bash scripts/ci/smoke-api.sh
```

### R3. Web Real-Browser QA

Status: completed.

Tasks:

- Drive `/repo/source-prism-ci` in a real browser after indexing.
- Verify files, symbols, references, impact, tests, docs/contracts, coverage,
  search, runs, and sync panes render without overlap or empty failed states.
- Capture desktop and mobile screenshots.

Evidence:

- Playwright screenshot artifacts under `.omo/evidence/`.
- Browser-driven assertions for key panels.
- Browser QA artifact: `.omo/evidence/web-real-browser-qa.json`.
- Desktop screenshot: `.omo/evidence/web-real-browser-desktop.png`.
- Mobile screenshot: `.omo/evidence/web-real-browser-mobile.png`.
- Verified indexed repo metrics, files, symbols, references, impact, tests,
  coverage empty-state, docs/contracts, runs, sync, search, and changed-symbols
  interactions against `/repo/source-prism-ci`.

### R4. OpenSearch Drift Repair UX

Status: completed.

Tasks:

- Make API/UI expose rebuild guidance when drift exists.
- Keep `search drift-check` non-zero on mismatch.
- Add user-facing recovery path: rebuild from Postgres, re-run worker,
  re-check drift.

Evidence:

- CLI drift mismatch fixture.
- API smoke for `has_drift=true` with remediation metadata.

### R5. Incremental PR Overlay Path

Status: completed.

Tasks:

- Promote overlay model into API/CLI changed-file indexing path.
- Prove changed files can be indexed without full repo re-index.
- Keep base commit canonical and head overlay separate.
- `changed-symbols` API/CLI now report changed-file overlay status for
  added, modified, deleted, renamed, and mode-only file diffs alongside
  impacted symbols.
- `changed-symbols` API/CLI can persist head overlay file evidence under
  `file_overlays` while leaving the base `file_manifest` generation count
  unchanged.

Evidence:

- Tests for added, modified, deleted, renamed, and mode-only files.
- CLI/API smoke showing overlay input and impacted symbols.
- CLI/API integration tests assert persisted overlay row count increases while
  base generation count stays unchanged.
- Real CLI smoke with `ri-cli changed-symbols --repo-id ... --persist-overlay`.
- Real HTTP smoke with `POST /v1/repos/{repo_id}/changed-symbols` and
  `persist_overlay: true`.

### R6. Production Hardening

Status: partial.

Tasks:

- Add auth/tenancy design before any non-local deployment mode.
- Add rate limits and request size limits for API.
- Add explicit secrets redaction for logs and review payloads.
- Add durable job observability endpoints and dead-letter inspection.
- API startup and config validation now refuse public/non-loopback
  `API_BIND_ADDR` values, keeping the current build local-only until
  auth/tenancy exists.
- Dead-letter inspection now exists through `GET /v1/repos/{repo_id}/dead-letters`
  and `ri-cli dead-letters --repo-id <repo_id>`.
- API JSON request bodies are capped at 256 KiB and oversized requests are
  rejected with HTTP 413 before route logic runs.
- Generation-scoped search drift checks now refresh OpenSearch, count only
  documents for that generation's repo/generation pair, and compare distinct
  document IDs so duplicate outbox upserts do not create false drift.
- Review dry-run payloads now redact secret-like `token=...`, `password=...`,
  GitHub/GitLab token prefixes, and `Authorization: Bearer ...` text before
  publishing artifacts are serialized.

Evidence:

- Security review notes.
- Config/API bind tests for public address rejection.
- Real CLI config-check and API startup smoke for public bind rejection.
- API tests for oversized/invalid requests.
- Worker tests for dead-letter visibility.
- `ri-indexer` drift regression for duplicate upserts.
- `ri-review`, `ri-github`, and `ri-gitlab` redaction regressions.

## Completion Criteria

The active goal can be marked complete only when current evidence proves:

- All required public interfaces above work against current code.
- CLI/API/worker/Web/MCP surfaces have real-surface verification.
- Postgres/OpenSearch drift checks pass after indexing and worker sync.
- Review and refactor stay inside their safety contracts.
- No target repo code execution occurs.
- Full cargo and SQLx gates pass.
- CI smoke is stable on a fresh run.
- Remaining dirty files are either committed intentionally or explicitly
  identified as user-owned.
