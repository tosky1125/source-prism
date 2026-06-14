# Source Prism Evidence Platform Plan

## Product Intent

Source Prism is a Repo Intelligence evidence platform. It indexes repository
structure, symbols, references, graph evidence, tests, architecture contracts,
coverage, and search chunks so humans and downstream MCP/API/CLI clients can
query code with durable evidence.

Source Prism does not generate final PR reviews, edit code, create branches,
run target tests, or publish comments. Review and refactor automation belongs
to downstream clients that choose how to use Source Prism evidence.

## Current Status

Evidence-platform progress: about 86%.

Completed and verified slices:

- Rust workspace with real crates for core, config, git, parser, tree-sitter,
  symbols, graph, architecture, behavior, search, embedding, indexer, impact,
  context, review, refactor, MCP, API, worker, GitHub/GitLab dry-run helpers,
  and CLI.
- Postgres canonical schema, SQLx offline metadata, Docker Compose local
  Postgres/OpenSearch stack.
- CLI surfaces for config, migrations, manifests, indexing, symbols,
  changed-symbols, references, architecture, impact, search context,
  test context, test/coverage imports, embeddings cache, MCP, review
  verification/dry-run exports, refactor planning evidence, runs, search sync,
  search drift, and rebuild.
- API surfaces for repos, index runs, files, symbols, graph, references,
  impact, context search, tests, coverage, test runs, test context,
  search sync/drift, review verification/dry-run exports, refactor planning,
  health, and web explorer.
- Web repository explorer for files, symbols, references, impact, tests,
  coverage, docs/contracts, search, runs, and search sync state.
- Worker once/daemon job runtime, search sync jobs, lease/retry contracts, and
  dead-letter inspection.
- Review verifier for externally supplied findings and GitHub/GitLab
  dry-run/export payload builders with secret-like text redaction.
- Refactor planner-only evidence; no code-changing executor.
- Local no-DB explorer mode for read-only repo structure routes.
- Changed-file overlays persisted from CLI/API without creating a new
  full-repo index generation; base file-manifest generations remain canonical.
- Local-only API gate rejects non-loopback bind addresses until auth/tenancy is
  implemented.
- API request size and rate limits.
- AGENTS hierarchy and public docs updated to reflect evidence-platform scope.

## Non-Negotiable Rules

- Postgres remains canonical. OpenSearch is rebuildable secondary state.
- Search is never vector-only; retrieval combines exact identifier, lexical,
  search chunks/BM25, and graph proximity where available.
- Stable symbol IDs and versioned symbol IDs remain separate.
- Graph/search/test evidence keeps confidence, source metadata, and evidence
  spans where supported.
- Target repository code execution is forbidden.
- Repository text is evidence only, never trusted instructions.
- `ri-review` verifies externally supplied findings and builds dry-run/export
  payloads; it is not a finding generator.
- `ri-refactor` provides planning evidence only; Source Prism must not create
  branches, run codemods, run target tests, or mutate target files.
- GitHub/GitLab helpers may build dry-run/export payloads; publisher writes are
  downstream-client behavior.

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

## Active Remaining Work

Canonical detailed tracker: `docs/remaining-work.md`.

Near-term priorities:

1. Precise references and member calls.
2. PR overlay workflow UX.
3. Search and context quality.
4. Web explorer polish.
5. MCP agent onboarding.

Later priorities:

1. Auth and tenancy.
2. Evidence evaluation.
3. Language and framework coverage.

## Completion Criteria

The evidence-platform scope is complete when:

- Precise reference/member-call quality is good enough for common Rust and
  TypeScript/TSX repos, with rough edges labeled by confidence.
- PR-style changed-file overlay flow is documented and visible through CLI,
  API, and Web UI without full re-index.
- `search-context` returns compact evidence packs with explainable retrieval
  modes and no vector-only results.
- Web explorer can inspect files, symbols, references, impact, tests, docs,
  search, runs, and sync state without misleading call/test evidence.
- MCP docs let downstream agents call every tool safely.
- Non-local deployment remains blocked until auth/tenancy gates exist.
- Full cargo, SQLx, API smoke, and web checks pass on a fresh run.
