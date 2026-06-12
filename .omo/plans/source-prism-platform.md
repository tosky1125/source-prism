# Source Prism Foundation Plan

## Status

Superseded by `.omo/plans/source-prism-full-platform.md`.

This foundation plan is retained as historical execution context. Its unchecked
todo boxes are no longer the active tracker: the foundation milestone has been
implemented and then expanded into the full-platform tracker. Current
completion status, remaining work, and verification evidence are recorded in
the full-platform plan and `.omo/evidence/full-platform-completion-audit.md`.

## TL;DR
> Summary:      Bootstrap Source Prism from an empty seed repo into a runnable Rust workspace for a Repo Intelligence Platform. Keep milestone 1 focused on foundation: git/workspace, typed core contracts, config, Postgres schema, job model, Git manifest/overlay contracts, CLI/API/worker smoke surfaces, local infra, CI, and evidence policy.
> Deliverables:
> - Git-initialized Rust workspace with initial crates: `ri-core`, `ri-config`, `ri-git`, `ri-indexer`, `ri-cli`, `ri-api`, `ri-worker`
> - Docker Compose local Postgres/OpenSearch stack plus `.env.example`
> - SQLx migrations and checked-in `.sqlx` policy
> - Typed IDs, confidence/evidence/trust-boundary contracts
> - Job contract, generation/stale-retirement schema, OpenSearch sync/outbox contract
> - CLI/API/worker smoke commands with agent-executed QA evidence
> - CI gate for fmt, clippy, tests, SQLx prepare, migration smoke
> - Roadmap-only waves for symbols, graph, search, SCIP/LSP, MCP, review/refactor
> Effort:       Large
> Risk:         High - empty repo plus DB/search/job/security boundaries

## Scope
### Must have

- Initialize the repository as a git repo because current state has no `.git` and `git status` fails.
- Create only the initial crates whose milestone-1 boundary is real:
  - `ri-core`: IDs, enums, evidence spans, confidence, trust-level types
  - `ri-config`: environment/config parsing and validation
  - `ri-git`: local Git/worktree manifest and overlay contracts
  - `ri-indexer`: generation lifecycle, stale-retirement, sync/outbox contract
  - `ri-cli`: smoke commands and foundation admin commands
  - `ri-api`: local-only health/status API
  - `ri-worker`: no-op/job-contract worker smoke path
- Keep API unauthenticated and local-only in milestone 1. Bind to `127.0.0.1` by default. Real auth, tenancy, repo tokens, GitHub/GitLab tokens, and secret storage are out of scope.
- Use Docker Compose as the default local infra path for Postgres + OpenSearch. Also support externally supplied `DATABASE_URL` and `OPENSEARCH_URL` through `.env.example`.
- Use Postgres as canonical store. OpenSearch is secondary and eventually consistent through idempotent sync/outbox jobs.
- Define OpenSearch rebuild/drift-check commands even if full chunk indexing is roadmap-only.
- Use SQLx migrations and checked-in `.sqlx` metadata. CI runs both live migration checks and `cargo sqlx prepare --workspace --check`.
- Establish error model: `thiserror` for library crates, `anyhow` at app boundaries, no `unwrap`/`panic` in production paths, deterministic CLI exit codes, JSON API error shape, tracing context on failures.
- Establish security boundary: foundation never executes target repository code. Git/index paths are read-only with no network execution. Test/refactor sandboxes are roadmap-only and prohibited until explicitly implemented.
- Establish prompt-injection boundary types now: source text has `TrustLevel::Untrusted`, evidence spans, and cannot be rendered as system/developer instructions.
- Include exact QA commands and evidence paths for every todo.

### Must NOT have

- Do not implement full symbol extraction, graph impact analysis, hybrid retrieval, SCIP/LSP resolution, MCP tools, GitHub/GitLab publishing, review generation, or refactor execution in milestone 1.
- Do not create empty final-shape crates (`ri-parser`, `ri-tree-sitter`, `ri-scip`, `ri-lsp`, `ri-graph`, `ri-search`, `ri-embedding`, `ri-context`, `ri-review`, `ri-refactor`, `ri-mcp`, `ri-github`, `ri-gitlab`, `ri-eval`) unless a later approved plan gives them runnable/testable responsibility.
- Do not use CocoIndex, GitNexus, Sourcegraph, or any full repo-intelligence framework.
- Do not hand-roll Git parser, Tree-sitter grammars, or a vector search engine.
- Do not publish or execute untrusted PR code. Milestone 1 may parse repo metadata only.
- Do not make OpenSearch canonical. Rebuild from Postgres must remain possible.
- Do not auto-commit unless the user separately asks for commits during execution.

## Verification strategy
> Zero human intervention - all verification is agent-executed.

- Test decision: TDD for domain/schema/overlay/job invariants; tests-after for thin CLI/API/worker smoke wiring.
- QA policy: every todo has agent-executed scenarios with exact command/API invocation.
- Evidence root: `.omo/evidence/`
- Evidence naming: `.omo/evidence/task-<N>-<slug>.<ext>`
- Baseline full gate after each implementation wave:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Database/search gate when infra exists:

```bash
docker compose up -d postgres opensearch
DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx migrate run
DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx prepare --workspace --check
```

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. < 3 per wave (except the final) = under-splitting.

Dependency note: the first two waves are intentionally small because current repo has no git or Cargo workspace. Do not fake parallelism before those prerequisites exist.

Wave 1 (no deps): T1
Wave 2 (after T1): T2, T5
Wave 3 (after T2): T3, T4, T6, T7
Wave 4 (after T3/T4/T6/T7): T8, T9, T11, T12
Wave 5 (after T8/T9/T11/T12): T10, T13, T14, T15, T16
Wave 6 (after T10/T13/T14/T15/T16): T17, T18, T19
Wave 7 (after T17/T18/T19): T20
Wave 8 (after T20): T21
Final verification wave: F1-F4 after all todos

Critical path: T1 -> T2 -> T3/T4/T6/T7 -> T8/T9/T11/T12 -> T10/T13/T14/T15/T16 -> T17/T18/T19 -> T20 -> T21 -> final verification.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
|------|------------|--------|----------------------|
| T1 | none | T2, T5 | none |
| T2 | T1 | T3, T4, T6, T7, T11, T12 | T5 |
| T3 | T2 | T8, T11, T12, T13, T14 | T4, T6, T7 |
| T4 | T2 | T8, T15 | T3, T6, T7 |
| T5 | T1 | T17, T18, T21 | T2 |
| T6 | T2 | T8, T17 | T3, T4, T7 |
| T7 | T2 | T8, T9, T10 | T3, T4, T6 |
| T8 | T3, T4, T6, T7 | T10, T13, T15, T18 | T9, T11, T12 |
| T9 | T7 | T10, T16, T18 | T8, T11, T12 |
| T10 | T8, T9 | T18 | T13, T15, T16, T19 |
| T11 | T2, T3 | T13, T17 | T8, T9, T12 |
| T12 | T2, T3 | T13, T14 | T8, T9, T11 |
| T13 | T8, T11, T12 | T17, T18 | T10, T15, T16, T19 |
| T14 | T3, T12 | T17, T19 | T10, T13, T15, T16 |
| T15 | T4, T8 | T17, T18, T21 | T10, T13, T16, T19 |
| T16 | T9 | T18, T21 | T10, T13, T15, T19 |
| T17 | T6, T11, T13, T14, T15 | T21 | T18, T19 |
| T18 | T8, T9, T10, T13, T15, T16 | T21 | T17, T19 |
| T19 | T14 | T20, T21 | T17, T18 |
| T20 | T19 | T21 | none |
| T21 | T17, T18, T20 | F1-F4 | none |

## Todos
> Implementation + Test = ONE todo. Never separate.

- [ ] T1. Initialize git repo and root project hygiene
  What to do / Must NOT do: Run `git init`; add `.gitignore`, `README.md`, `.editorconfig`, `rust-toolchain.toml`, and root docs for local-only milestone policy. Do not commit unless user explicitly asks.
  Parallelization: Can parallel N | Wave 1 | Blocks T2, T5
  References: `AGENTS.md:10`, `AGENTS.md:30`, `AGENTS.md:143`, `.omo/drafts/source-prism-platform.md:31-42`
  Acceptance criteria: `test -d .git`; `git status --short --branch` exits 0; README states Source Prism is Rust-first Repo Intelligence Platform and milestone 1 is foundation only.
  QA scenarios:
  - happy: shell `test -d .git && git status --short --branch > .omo/evidence/task-1-git-status.txt`
  - failure: shell `grep -n "foundation" README.md > .omo/evidence/task-1-readme-foundation.txt`
  Commit: Y | `chore(repo): initialize project hygiene` | `.gitignore`, `README.md`, `.editorconfig`, `rust-toolchain.toml`

- [ ] T2. Create Rust workspace and initial crate skeletons
  What to do / Must NOT do: Add root `Cargo.toml` workspace and only initial crates: `ri-core`, `ri-config`, `ri-git`, `ri-indexer`, `ri-cli`, `ri-api`, `ri-worker`. Each crate must have runnable/testable responsibility; no empty roadmap crates.
  Parallelization: Can parallel Y | Wave 2 | Blocks T3, T4, T6, T7, T11, T12
  References: `AGENTS.md:12-18`, `AGENTS.md:28-61`, `.omo/drafts/source-prism-platform.md:145-148`
  Acceptance criteria: `cargo metadata --format-version 1` lists exactly initial crates; no `crates/ri-mcp`, `crates/ri-parser`, `crates/ri-tree-sitter`, `crates/ri-scip`, `crates/ri-lsp`, `crates/ri-graph`, `crates/ri-search`, `crates/ri-review`.
  QA scenarios:
  - happy: shell `cargo metadata --format-version 1 > .omo/evidence/task-2-cargo-metadata.json`
  - exact crate set: shell `bash -lc 'find crates -mindepth 1 -maxdepth 1 -type d -printf "%f\\n" | sort > .omo/evidence/task-2-crates.txt; printf "ri-api\\nri-cli\\nri-config\\nri-core\\nri-git\\nri-indexer\\nri-worker\\n" > .omo/evidence/task-2-expected-crates.txt; diff -u .omo/evidence/task-2-expected-crates.txt .omo/evidence/task-2-crates.txt'`
  Commit: Y | `chore(workspace): scaffold initial crates` | `Cargo.toml`, `crates/**`

- [ ] T3. Implement `ri-core` domain contracts with TDD
  What to do / Must NOT do: Add newtypes for repo/commit/file/symbol/entity/edge/chunk/job/generation IDs; language/symbol/edge enums; evidence spans; trust levels; confidence tiers `Exact`, `High`, `Medium`, `Low`; deterministic ID helpers. Use `thiserror`; no broad stringly typed IDs in public APIs.
  Parallelization: Can parallel Y | Wave 3 | Blocks T8, T11, T12, T13, T14
  References: `AGENTS.md:33-58`, `AGENTS.md:80-89`, `.omo/drafts/source-prism-platform.md:46-49`, `.omo/drafts/source-prism-platform.md:115-116`
  Acceptance criteria: unit tests prove deterministic IDs are stable for identical inputs and differ for changed commit/content/evidence; confidence tiers serialize/deserialize; untrusted evidence cannot be converted into instructions by type API.
  QA scenarios:
  - happy: shell `cargo test -p ri-core -- --nocapture > .omo/evidence/task-3-ri-core-tests.txt`
  - panic guard: shell `bash -lc '(rg "unwrap\\(|panic!\\(" crates/ri-core/src > .omo/evidence/task-3-forbidden-panics.txt || true); test ! -s .omo/evidence/task-3-forbidden-panics.txt'`
  Commit: Y | `feat(core): add typed repo intelligence contracts` | `crates/ri-core/**`

- [ ] T4. Implement `ri-config` validation and error model
  What to do / Must NOT do: Add config structs for database, OpenSearch, API bind address, worker, evidence dir, and feature gates. Default API bind must be `127.0.0.1`. Errors use typed library errors and app-boundary rendering.
  Parallelization: Can parallel Y | Wave 3 | Blocks T8, T15
  References: `AGENTS.md:17`, `AGENTS.md:53-54`, `AGENTS.md:139`, `.omo/drafts/source-prism-platform.md:88-96`
  Acceptance criteria: tests cover missing required env, invalid URLs, local-only API default, and deterministic redaction of secret-looking values.
  QA scenarios:
  - happy: shell `cargo test -p ri-config -- --nocapture > .omo/evidence/task-4-ri-config-tests.txt`
  - failure: shell `bash -lc 'set +e; cargo run -p ri-cli -- config check --env-file /tmp/source-prism-missing.env > .omo/evidence/task-4-config-failure.txt 2>&1; code=$?; echo $code > .omo/evidence/task-4-config-failure.exit; test "$code" -ne 0 && grep -qi "config" .omo/evidence/task-4-config-failure.txt && ! grep -E "password|secret|token" .omo/evidence/task-4-config-failure.txt'`
  Commit: Y | `feat(config): add validated runtime configuration` | `crates/ri-config/**`

- [ ] T5. Establish repository evidence and QA conventions
  What to do / Must NOT do: Create `.omo/evidence/.gitkeep`, document evidence naming, add a `justfile` or `Makefile` only if it wraps exact cargo/docker commands without hiding failures. Do not replace explicit commands with opaque scripts in acceptance criteria.
  Parallelization: Can parallel Y | Wave 2 | Blocks T17, T18, T21
  References: `AGENTS.md:80`, `.omo/drafts/source-prism-platform.md:76-82`, `.omo/drafts/source-prism-platform.md:103-104`
  Acceptance criteria: evidence directory exists; docs define RED/GREEN and real-surface evidence policy for future work.
  QA scenarios:
  - happy: shell `test -d .omo/evidence && find .omo/evidence -maxdepth 2 -type f | sort > .omo/evidence/task-5-evidence-layout.txt`
  - failure: shell `grep -n "real-surface" README.md > .omo/evidence/task-5-real-surface-policy.txt`
  Commit: Y | `docs(qa): define evidence conventions` | `README.md`, `.omo/evidence/.gitkeep`

- [ ] T6. Add local Postgres/OpenSearch development stack
  What to do / Must NOT do: Add `docker-compose.yml` with `postgres` and `opensearch` services, stable local ports, health checks, persistent named volumes, and `.env.example`. Docker Compose is default; external DB/search URLs are supported through config.
  Parallelization: Can parallel Y | Wave 3 | Blocks T8, T17
  References: `AGENTS.md:15`, `AGENTS.md:86-87`, `.omo/drafts/source-prism-platform.md:69-72`, `.omo/drafts/source-prism-platform.md:106-110`
  Acceptance criteria: `docker compose up -d postgres opensearch` starts healthy services; `.env.example` contains `DATABASE_URL` and `OPENSEARCH_URL`.
  QA scenarios:
  - happy: shell `bash -lc 'docker compose up -d postgres opensearch && docker compose ps > .omo/evidence/task-6-compose-ps.txt && docker compose exec -T postgres pg_isready -U source_prism > .omo/evidence/task-6-postgres-health.txt && curl -fsS http://localhost:9200/_cluster/health > .omo/evidence/task-6-opensearch-health.json'`
  - config guard: shell `bash -lc 'grep -E "DATABASE_URL=" .env.example > .omo/evidence/task-6-env-example.txt && grep -E "OPENSEARCH_URL=" .env.example >> .omo/evidence/task-6-env-example.txt'`
  Commit: Y | `chore(dev): add local data services` | `docker-compose.yml`, `.env.example`

- [ ] T7. Add SQLx migrations for canonical foundation schema
  What to do / Must NOT do: Add migrations for `repos`, `commits`, `file_manifests`, `index_generations`, `symbols`, `graph_nodes`, `graph_edges`, `jobs`, `job_attempts`, `search_sync_outbox`, `embedding_cache`. Include `generation_id` and `stale_at` where rows are refreshable. No hard-delete normal refresh path.
  Parallelization: Can parallel Y | Wave 3 | Blocks T8, T9, T10
  References: `AGENTS.md:91-95`, `.omo/drafts/source-prism-platform.md:47-49`, `.omo/drafts/source-prism-platform.md:109-110`
  Acceptance criteria: migrations run on clean Postgres; rerun is no-op; indexes exist for repo/commit/source/target lookups and job leases.
  QA scenarios:
  - happy: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx migrate run > .omo/evidence/task-7-migrate-run.txt`
  - failure: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx migrate run > .omo/evidence/task-7-migrate-rerun.txt`
  Commit: Y | `feat(db): add canonical foundation schema` | `migrations/**`

- [ ] T8. Establish SQLx offline and CI database policy
  What to do / Must NOT do: Add checked-in `.sqlx`; CI uses Postgres service for migrations and runs `cargo sqlx prepare --workspace --check`. Do not rely only on compile without schema.
  Parallelization: Can parallel Y | Wave 4 | Blocks T10, T13, T15, T18
  References: `.omo/drafts/source-prism-platform.md:53-55`, `.omo/drafts/source-prism-platform.md:69-72`, `AGENTS.md:101-105`
  Acceptance criteria: `.sqlx` exists after prepare; `cargo sqlx prepare --workspace --check` passes with clean DB; docs explain when to update `.sqlx`.
  QA scenarios:
  - happy: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx prepare --workspace --check > .omo/evidence/task-8-sqlx-prepare-check.txt`
  - artifact guard: shell `bash -lc 'test -d .sqlx && find .sqlx -type f | sort > .omo/evidence/task-8-sqlx-files.txt && test -s .omo/evidence/task-8-sqlx-files.txt'`
  Commit: Y | `build(db): enforce sqlx offline checks` | `.sqlx/**`, `.github/workflows/**`, docs

- [ ] T9. Implement durable job contract in schema and `ri-worker`
  What to do / Must NOT do: Define job state machine (`queued`, `leased`, `succeeded`, `failed`, `dead_lettered`, `cancelled`), idempotency key, priority, run-after, retry count, retry/backoff, lease timeout, cancellation, and worker identity. `ri-worker --once` must process no-op work deterministically.
  Parallelization: Can parallel Y | Wave 4 | Blocks T10, T16, T18
  References: `AGENTS.md:54`, `AGENTS.md:119`, `.omo/drafts/source-prism-platform.md:94-96`
  Acceptance criteria: unit/integration tests prove lease uniqueness, retry to dead-letter, cancellation, and idempotent enqueue.
  QA scenarios:
  - happy: shell `cargo test -p ri-worker -- --nocapture > .omo/evidence/task-9-worker-tests.txt`
  - failure: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo run -p ri-worker -- --once > .omo/evidence/task-9-worker-once.txt 2>&1`
  Commit: Y | `feat(worker): define durable job runtime contract` | `crates/ri-worker/**`, `migrations/**`

- [ ] T10. Add OpenSearch sync/outbox and drift-repair contract
  What to do / Must NOT do: Add canonical `search_sync_outbox` workflow and CLI admin commands for `search sync --once`, `search drift-check`, and `search rebuild --from-postgres`. Commands may be thin in milestone 1 but must use real DB rows and OpenSearch health checks, not fake success.
  Parallelization: Can parallel Y | Wave 5 | Blocks T18
  References: `AGENTS.md:86-87`, `AGENTS.md:123`, `.omo/drafts/source-prism-platform.md:109-110`
  Acceptance criteria: tests prove outbox rows are idempotent by deterministic key; rebuild command can delete/recreate a dev index from canonical rows; drift-check returns non-zero on mismatch.
  QA scenarios:
  - happy: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism OPENSEARCH_URL=http://localhost:9200 cargo run -p ri-cli -- search sync --once > .omo/evidence/task-10-search-sync-once.txt 2>&1`
  - drift mismatch: shell `bash -lc 'set +e; DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism OPENSEARCH_URL=http://localhost:9200 cargo run -p ri-cli -- search drift-check --expect-mismatch fixture > .omo/evidence/task-10-drift-check.txt 2>&1; code=$?; echo $code > .omo/evidence/task-10-drift-check.exit; test "$code" -ne 0 && grep -qi "drift" .omo/evidence/task-10-drift-check.txt'`
  Commit: Y | `feat(indexer): define search sync contract` | `crates/ri-indexer/**`, `crates/ri-cli/**`, `migrations/**`

- [ ] T11. Implement `ri-git` local manifest and content hash extraction
  What to do / Must NOT do: Use `gix` library-facing API first. Add `git2` only behind a feature-gated fallback boundary if an explicit fixture requires it. Produce manifest records for path, language guess, size, SHA-256 content hash, generated/vendor/test flags. Do not execute repository code.
  Parallelization: Can parallel Y | Wave 4 | Blocks T13, T17
  References: `AGENTS.md:15-17`, `AGENTS.md:36`, `AGENTS.md:68`, `AGENTS.md:132`, `.omo/drafts/source-prism-platform.md:55-58`
  Acceptance criteria: fixture tests cover empty repo, deleted file, binary file skip, generated/vendor detection, and deterministic hashes.
  QA scenarios:
  - happy: shell `cargo test -p ri-git -- --nocapture > .omo/evidence/task-11-ri-git-tests.txt`
  - command-exec guard: shell `bash -lc '(rg "std::process::Command|Command::new" crates/ri-git/src > .omo/evidence/task-11-no-command-exec.txt || true); test ! -s .omo/evidence/task-11-no-command-exec.txt'`
  Commit: Y | `feat(git): extract deterministic file manifests` | `crates/ri-git/**`

- [ ] T12. Define PR overlay data model without full symbol extraction
  What to do / Must NOT do: Add overlay types for added/modified/deleted/renamed/mode-only files and merged-view semantics. Do not implement Tree-sitter parsing yet. Define how base rows are shadowed by head rows and overlay deletions.
  Parallelization: Can parallel Y | Wave 4 | Blocks T13, T14
  References: `AGENTS.md:81-84`, `.omo/drafts/source-prism-platform.md:122-131`
  Acceptance criteria: tests prove overlay view does not mix stale base file records when head changes/deletes/renames a file.
  QA scenarios:
  - happy: shell `cargo test -p ri-indexer overlay -- --nocapture > .omo/evidence/task-12-overlay-tests.txt`
  - failure: shell `cargo test -p ri-indexer overlay_delete -- --nocapture > .omo/evidence/task-12-overlay-delete.txt`
  Commit: Y | `feat(indexer): model pr overlay semantics` | `crates/ri-indexer/**`

- [ ] T13. Implement generation lifecycle and soft stale-retirement service
  What to do / Must NOT do: Add begin/finish/fail generation APIs, attach `generation_id`, set `stale_at` for rows not regenerated, and preserve old rows for audit. No hard delete in normal refresh.
  Parallelization: Can parallel Y | Wave 5 | Blocks T17, T18
  References: `AGENTS.md:82`, `AGENTS.md:91-95`, `.omo/drafts/source-prism-platform.md:90-92`
  Acceptance criteria: integration tests prove successful generation marks missing previous rows stale and failed generation leaves previous active rows untouched.
  QA scenarios:
  - happy: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo test -p ri-indexer generation -- --nocapture > .omo/evidence/task-13-generation-tests.txt`
  - failure: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo test -p ri-indexer failed_generation -- --nocapture > .omo/evidence/task-13-failed-generation.txt`
  Commit: Y | `feat(indexer): add generation stale retirement` | `crates/ri-indexer/**`, `migrations/**`

- [ ] T14. Add prompt-injection trust boundary and context safety types
  What to do / Must NOT do: In `ri-core`, define `TrustLevel`, `EvidenceSpan`, `EvidenceSourceKind`, and untrusted text wrappers. Add compile-time API that prevents untrusted repo text from being formatted as trusted instructions without explicit unsafe-named conversion unavailable to normal code.
  Parallelization: Can parallel Y | Wave 5 | Blocks T17, T19
  References: `AGENTS.md:80`, `AGENTS.md:88-89`, `.omo/drafts/source-prism-platform.md:88-90`, `.omo/drafts/source-prism-platform.md:112-113`
  Acceptance criteria: tests prove context pack builder can include untrusted code/comment/doc text only as evidence with source, file path, line span, and trust level.
  QA scenarios:
  - happy: shell `cargo test -p ri-core trust -- --nocapture > .omo/evidence/task-14-trust-tests.txt`
  - trust-mixing guard: shell `bash -lc '(rg "trusted_instructions.*untrusted|system.*untrusted" crates -n > .omo/evidence/task-14-untrusted-search.txt || true); test ! -s .omo/evidence/task-14-untrusted-search.txt'`
  Commit: Y | `feat(core): add trust boundary evidence types` | `crates/ri-core/**`

- [ ] T15. Add local-only API health surface
  What to do / Must NOT do: Implement `ri-api` with Axum `Router::with_state`, `GET /v1/health`, and structured JSON errors. Bind to `127.0.0.1` by default. No auth/token system in milestone 1; document local-only scope.
  Parallelization: Can parallel Y | Wave 5 | Blocks T17, T18, T21
  References: `AGENTS.md:53`, `AGENTS.md:76`, `AGENTS.md:113`, `.omo/drafts/source-prism-platform.md:53`, `.omo/drafts/source-prism-platform.md:94`
  Acceptance criteria: API starts, `GET /v1/health` returns HTTP 200 and JSON with service, version, database status, opensearch status; unhealthy dependencies produce non-200 or degraded field by explicit contract.
  QA scenarios:
  - happy: shell `bash -lc 'set -euo pipefail; RUST_LOG=debug cargo run -p ri-api > .omo/evidence/task-15-api-run.log 2>&1 & pid=$!; echo $pid > .omo/evidence/task-15-api.pid; trap "kill $pid 2>/dev/null || true" EXIT; for i in $(seq 1 30); do curl -fsS http://127.0.0.1:3000/v1/health >/dev/null && break; sleep 1; done; curl -i http://127.0.0.1:3000/v1/health > .omo/evidence/task-15-api-health.txt; grep -q "HTTP/1.1 200 OK" .omo/evidence/task-15-api-health.txt; grep -q "\"service\":\"ri-api\"" .omo/evidence/task-15-api-health.txt'`
  - local-only guard: shell `bash -lc 'grep -R "127.0.0.1" crates/ri-api crates/ri-config > .omo/evidence/task-15-local-bind.txt'`
  Commit: Y | `feat(api): add local health endpoint` | `crates/ri-api/**`

- [ ] T16. Add worker smoke mode and no-op job processing
  What to do / Must NOT do: Implement `cargo run -p ri-worker -- --once` and `--poll-interval-ms`. In `--once`, acquire at most one job, process known no-op job type, record attempt, then exit. No long-running daemon in QA without cleanup.
  Parallelization: Can parallel Y | Wave 5 | Blocks T18, T21
  References: `AGENTS.md:54`, `AGENTS.md:114`, `.omo/drafts/source-prism-platform.md:81`, `.omo/drafts/source-prism-platform.md:94-96`
  Acceptance criteria: `--once` exits 0 when no work; seeded no-op job transitions to `succeeded`; failed job retries according to policy.
  QA scenarios:
  - happy: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo run -p ri-worker -- --once > .omo/evidence/task-16-worker-once.txt 2>&1`
  - failure: shell `DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo test -p ri-worker no_op_job -- --nocapture > .omo/evidence/task-16-noop-job-test.txt`
  Commit: Y | `feat(worker): add once-mode smoke processing` | `crates/ri-worker/**`

- [ ] T17. Add CLI foundation commands
  What to do / Must NOT do: Implement `ri-cli` commands: `config check`, `db migrate`, `repo manifest --repo .`, `index --repo . --sha WORKTREE`, `symbols --repo .`, `impact --symbol ...`, `search sync --once`, `search drift-check`. `symbols` and `impact` may return explicit `not_implemented` JSON until symbol milestone, but command shape must be stable and non-misleading.
  Parallelization: Can parallel Y | Wave 6 | Blocks T21
  References: `AGENTS.md:57`, `AGENTS.md:109-112`, `.omo/drafts/source-prism-platform.md:80-82`
  Acceptance criteria: commands exit deterministically; unimplemented feature commands return machine-readable status with roadmap milestone, not fake results; unimplemented feature commands exit with code 2.
  QA scenarios:
  - happy: shell `cargo run -p ri-cli -- repo manifest --repo . > .omo/evidence/task-17-manifest.txt 2>&1`
  - not-implemented guard: shell `bash -lc 'set +e; cargo run -p ri-cli -- impact --symbol "InvoiceService::applyTax" > .omo/evidence/task-17-impact-not-implemented.json 2>&1; code=$?; echo $code > .omo/evidence/task-17-impact-not-implemented.exit; test "$code" -eq 2 && grep -q "not_implemented" .omo/evidence/task-17-impact-not-implemented.json'`
  Commit: Y | `feat(cli): add foundation command surface` | `crates/ri-cli/**`

- [ ] T18. Add CI workflow with full foundation gate
  What to do / Must NOT do: Add `.github/workflows/ci.yml` with Rust cache, Docker services or service containers for Postgres/OpenSearch, migrations, SQLx prepare check, fmt, clippy, tests, CLI/API/worker smoke where feasible. Do not allow CI to pass if smoke commands are skipped silently.
  Parallelization: Can parallel Y | Wave 6 | Blocks T21
  References: `AGENTS.md:101-105`, `.omo/drafts/source-prism-platform.md:69-72`
  Acceptance criteria: `act` compatibility is optional, but workflow YAML validates; local commands matching CI pass.
  QA scenarios:
  - happy: shell `cargo fmt --all -- --check > .omo/evidence/task-18-fmt.txt && cargo clippy --workspace --all-targets -- -D warnings > .omo/evidence/task-18-clippy.txt && cargo test --workspace > .omo/evidence/task-18-tests.txt`
  - workflow guard: shell `bash -lc 'test -f .github/workflows/ci.yml && grep -n "cargo clippy" .github/workflows/ci.yml > .omo/evidence/task-18-ci-yaml.txt && grep -n "cargo sqlx prepare" .github/workflows/ci.yml >> .omo/evidence/task-18-ci-yaml.txt'`
  Commit: Y | `ci: add foundation quality gate` | `.github/workflows/ci.yml`

- [ ] T19. Add security and sandbox policy document
  What to do / Must NOT do: Document milestone-1 enforcement: no target repo code execution; no secrets in index path; no network access for future sandboxed execution by default; test/refactor execution deferred until sandbox design. Include prompt-injection separation and MCP security as roadmap concerns.
  Parallelization: Can parallel Y | Wave 6 | Blocks T20, T21
  References: `AGENTS.md:88-89`, `AGENTS.md:138`, `.omo/drafts/source-prism-platform.md:88-90`, `.omo/drafts/source-prism-platform.md:112-113`
  Acceptance criteria: docs define allowed/forbidden operations for indexing, LSP/SCIP, test execution, refactor execution; milestone 1 explicitly forbids execution of target repo code.
  QA scenarios:
  - happy: shell `grep -n "must not execute target repository code" docs/security.md > .omo/evidence/task-19-no-exec-policy.txt`
  - failure: shell `grep -n "untrusted" docs/security.md > .omo/evidence/task-19-untrusted-policy.txt`
  Commit: Y | `docs(security): define foundation trust boundary` | `docs/security.md`

- [ ] T20. Add roadmap guardrail document for post-foundation waves
  What to do / Must NOT do: Document roadmap-only waves for symbol extraction, graph/impact, search/chunks, SCIP/LSP, behavior/test evidence, MCP, GitHub/GitLab, review/refactor. Explicitly say they are not milestone-1 implementation tasks.
  Parallelization: Can parallel N | Wave 7 | Blocks T21
  References: `AGENTS.md:117-128`, `.omo/drafts/source-prism-platform.md:122-131`
  Acceptance criteria: roadmap maps each future crate to entry condition, required evidence, and why not created in foundation.
  QA scenarios:
  - happy: shell `grep -n "Roadmap only" docs/roadmap.md > .omo/evidence/task-20-roadmap-only.txt`
  - failure: shell `grep -n "ri-mcp" docs/roadmap.md > .omo/evidence/task-20-mcp-roadmap.txt`
  Commit: Y | `docs(roadmap): separate foundation from platform waves` | `docs/roadmap.md`

- [ ] T21. Run full foundation real-surface QA and cleanup
  What to do / Must NOT do: Drive CLI, API, worker, DB, OpenSearch through real surfaces. Capture command outputs. Stop services/processes after QA and record cleanup receipts.
  Parallelization: Can parallel N | Wave 8 | Blocks final verification
  References: `AGENTS.md:107-115`, `.omo/drafts/source-prism-platform.md:103-104`
  Acceptance criteria: all commands below pass; evidence files exist; `docker compose down` or cleanup commands run and are recorded.
  QA scenarios:
  - CLI: shell `bash -lc 'cargo run -p ri-cli -- repo manifest --repo . > .omo/evidence/task-21-cli-manifest.txt 2>&1; grep -qi "manifest" .omo/evidence/task-21-cli-manifest.txt'`
  - API: shell `bash -lc 'set -euo pipefail; RUST_LOG=info cargo run -p ri-api > .omo/evidence/task-21-api.log 2>&1 & pid=$!; echo $pid > .omo/evidence/task-21-api.pid; trap "kill $pid 2>/dev/null || true" EXIT; for i in $(seq 1 30); do curl -fsS http://127.0.0.1:3000/v1/health >/dev/null && break; sleep 1; done; curl -i http://127.0.0.1:3000/v1/health > .omo/evidence/task-21-api-health.txt; grep -q "HTTP/1.1 200 OK" .omo/evidence/task-21-api-health.txt'`
  - Worker: shell `bash -lc 'DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo run -p ri-worker -- --once > .omo/evidence/task-21-worker-once.txt 2>&1; grep -Eqi "no work|processed|succeeded" .omo/evidence/task-21-worker-once.txt'`
  - Cleanup: shell `bash -lc 'docker compose down > .omo/evidence/task-21-cleanup.txt 2>&1; grep -Eqi "Removing|Network|Container|done|Stopped" .omo/evidence/task-21-cleanup.txt || test -s .omo/evidence/task-21-cleanup.txt'`
  Commit: N | final QA only | evidence files

## Roadmap-only waves after foundation

These waves are intentionally non-executable in this plan. Create separate approved plans before implementation. Each future plan must restate entry conditions, evidence, and exact QA before any crate is created.

| Roadmap wave | Entry condition | Required evidence before implementation | Acceptance criteria for that later plan |
|--------------|-----------------|------------------------------------------|-----------------------------------------|
| Symbol index: `ri-parser`, `ri-tree-sitter`, `ri-symbols` | Foundation T1-T21 and F1-F4 complete | Tree-sitter binding/grammar versions pinned; fixture repos chosen for Rust/TypeScript/Python/PHP at minimum | Changed-line to symbol mapping target defined; fixture RED/GREEN tests prove deterministic symbol IDs and range accuracy |
| Graph/impact: `ri-graph`, `ri-impact` | Symbol records and file manifests exist | Edge confidence taxonomy from foundation types is used unchanged or explicitly migrated | Impact traversal tests cover reverse calls/imports/routes, evidence paths, and no low-confidence hidden promotion |
| Search/retrieval: `ri-search`, `ri-embedding` | Postgres canonical rows and OpenSearch outbox contract pass drift tests | Embedding provider selected behind adapter; content-hash cache and retry budget defined | Retrieval acceptance includes exact identifier, BM25, vector, graph boost, and a test proving vector-only ranking is rejected |
| Precise refs: `ri-scip`, `ri-lsp` | Symbol index fixtures pass | SCIP tool coverage and LSP server strategy documented per language; resolver conflict policy chosen | Resolver confidence thresholds defined: `Exact`/`High` may influence review context; `Medium` searchable only; `Low` diagnostic only unless verifier upgrades |
| Architecture/test behavior: `ri-architecture`, `ri-behavior` | Graph schema stable | OpenAPI/GraphQL/DB/event/test artifacts selected as fixtures | Coverage-to-symbol confidence levels tested; missing-test output includes evidence and refuses claims without coverage/name/path support |
| MCP/agent integration: `ri-mcp` | CLI/API query surfaces stable and security policy reviewed | MCP spec version and Rust SDK pinned; prompt-injection model reviewed | Tool schemas validate inputs; untrusted repo text never appears as instructions; stdio/process execution threat model documented |
| GitHub/GitLab publishing: `ri-github`, `ri-gitlab` | Auth/tenancy/token model approved | Token storage/redaction/audit design reviewed; repo/org isolation model documented | Publisher refuses findings without file/line/evidence/impact path/actionability; no secret access for untrusted PR code |
| Review/refactor/eval: `ri-context`, `ri-review`, `ri-refactor`, `ri-eval` | Context, graph, search, behavior evidence available | Golden dataset/replay corpus selected; evaluator metrics defined | Eval metrics include useful finding rate, false positive rate, duplicate rate, line accuracy, severity calibration, symbol/reference precision/recall, impact path precision, latency, and reproducibility |

## Final verification wave (after ALL todos)
> Runs in parallel. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.

- [ ] F1. Plan compliance audit
  Verify implementation touched only foundation scope; no roadmap-only crates created; no full framework dependencies added.
  Invocation: `multi_agent_v1.spawn_agent({"agent_type":"codex-ultrawork-reviewer","fork_context":false,"message":"TASK: audit implementation against .omo/plans/source-prism-platform.md. DELIVERABLE: final line APPROVED or REJECTED. VERIFY: APPROVED only if no roadmap-only crates exist, no full framework dependency appears, and all T1-T21 evidence files exist; otherwise REJECTED with paths."})`; save reviewer final output to `.omo/evidence/f1-plan-compliance.md`.
  Binary observable: `.omo/evidence/f1-plan-compliance.md` final line is exactly `APPROVED`.
  Evidence: `.omo/evidence/f1-plan-compliance.md`

- [ ] F2. Code quality review
  Run read-only review for Rust type safety, error handling, no `unwrap`/`panic`, no code execution in index path, no hidden skipped tests.
  Invocation: shell `bash -lc 'cargo fmt --all -- --check > .omo/evidence/f2-fmt.txt && cargo clippy --workspace --all-targets -- -D warnings > .omo/evidence/f2-clippy.txt && cargo test --workspace > .omo/evidence/f2-tests.txt && (rg "unwrap\\(|panic!\\(|\\.skip\\(|\\.only\\(|xfail" crates > .omo/evidence/f2-forbidden.txt || true) && test ! -s .omo/evidence/f2-forbidden.txt && printf "APPROVED\\n" > .omo/evidence/f2-code-quality.md'`
  Binary observable: shell exits 0 and `.omo/evidence/f2-code-quality.md` contains exactly `APPROVED`.
  Evidence: `.omo/evidence/f2-code-quality.md`, `.omo/evidence/f2-fmt.txt`, `.omo/evidence/f2-clippy.txt`, `.omo/evidence/f2-tests.txt`, `.omo/evidence/f2-forbidden.txt`

- [ ] F3. Real manual QA
  Re-run T21 exact CLI/API/worker scenarios from a clean shell after `docker compose up -d postgres opensearch`.
  Invocation: shell `bash -lc 'set -euo pipefail; docker compose up -d postgres opensearch; cargo run -p ri-cli -- repo manifest --repo . > .omo/evidence/f3-cli-manifest.txt 2>&1; RUST_LOG=info cargo run -p ri-api > .omo/evidence/f3-api.log 2>&1 & pid=$!; echo $pid > .omo/evidence/f3-api.pid; trap "kill $pid 2>/dev/null || true; docker compose down > .omo/evidence/f3-cleanup.txt 2>&1 || true" EXIT; for i in $(seq 1 30); do curl -fsS http://127.0.0.1:3000/v1/health >/dev/null && break; sleep 1; done; curl -i http://127.0.0.1:3000/v1/health > .omo/evidence/f3-api-health.txt; grep -q "HTTP/1.1 200 OK" .omo/evidence/f3-api-health.txt; DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo run -p ri-worker -- --once > .omo/evidence/f3-worker-once.txt 2>&1; printf "APPROVED\\n" > .omo/evidence/f3-real-qa.md'`
  Binary observable: shell exits 0; `.omo/evidence/f3-real-qa.md` contains exactly `APPROVED`; cleanup evidence file exists after trap.
  Evidence: `.omo/evidence/f3-real-qa.md`, `.omo/evidence/f3-cli-manifest.txt`, `.omo/evidence/f3-api-health.txt`, `.omo/evidence/f3-worker-once.txt`, `.omo/evidence/f3-cleanup.txt`

- [ ] F4. Scope fidelity
  Compare delivered files against `AGENTS.md`, this plan, and approved defaults. Confirm auth/tenancy/MCP/parser/search/review/refactor remain roadmap-only unless explicitly implemented as accepted foundation contracts.
  Invocation: shell `bash -lc 'find . -maxdepth 3 -type d \\( -path "./crates/ri-mcp" -o -path "./crates/ri-parser" -o -path "./crates/ri-tree-sitter" -o -path "./crates/ri-scip" -o -path "./crates/ri-lsp" -o -path "./crates/ri-graph" -o -path "./crates/ri-search" -o -path "./crates/ri-review" -o -path "./crates/ri-refactor" \\) -print > .omo/evidence/f4-roadmap-crates.txt; test ! -s .omo/evidence/f4-roadmap-crates.txt; (rg "CocoIndex|GitNexus|Sourcegraph" Cargo.toml crates docs > .omo/evidence/f4-framework-search.txt || true); test ! -s .omo/evidence/f4-framework-search.txt; printf "APPROVED\\n" > .omo/evidence/f4-scope-fidelity.md'`
  Binary observable: shell exits 0; `.omo/evidence/f4-scope-fidelity.md` contains exactly `APPROVED`; both evidence search files are empty.
  Evidence: `.omo/evidence/f4-scope-fidelity.md`, `.omo/evidence/f4-roadmap-crates.txt`, `.omo/evidence/f4-framework-search.txt`

## Commit strategy

- Do not auto-commit unless the user explicitly asks during execution.
- If committing is approved later, use atomic Conventional Commits:
  - `chore(repo): initialize project hygiene`
  - `chore(workspace): scaffold initial crates`
  - `feat(core): add typed repo intelligence contracts`
  - `feat(config): add validated runtime configuration`
  - `chore(dev): add local data services`
  - `feat(db): add canonical foundation schema`
  - `feat(worker): define durable job runtime contract`
  - `feat(indexer): model pr overlay semantics`
  - `feat(api): add local health endpoint`
  - `feat(cli): add foundation command surface`
  - `ci: add foundation quality gate`
  - `docs(security): define foundation trust boundary`
- Every implementation commit must pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Success criteria

- Repository is initialized with git and has reproducible root hygiene.
- Workspace contains only approved initial crates, each with runnable/testable responsibility.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass.
- Docker Compose brings up Postgres and OpenSearch; `.env.example` documents external URL fallback.
- SQLx migrations run on clean Postgres; `.sqlx` is checked and CI enforces `cargo sqlx prepare --workspace --check`.
- Typed domain contracts cover deterministic IDs, evidence spans, trust levels, confidence tiers, and errors.
- Job contract covers state machine, retries, lease timeout, cancellation, idempotency, dead-lettering.
- Postgres is canonical; OpenSearch sync/outbox, drift-check, and rebuild contracts exist.
- CLI smoke commands run and return deterministic real or explicit `not_implemented` JSON.
- `GET /v1/health` returns concrete JSON over HTTP.
- `ri-worker --once` exits 0 on no work and processes no-op job in tests.
- Security docs prohibit target repo code execution in milestone 1 and separate untrusted context from instructions.
- Roadmap-only waves are documented and not implemented in foundation.
- Final verification F1-F4 all approve with evidence.

## Plan generation acceptance

- This planning turn may create/update only `.omo/drafts/source-prism-platform.md` and `.omo/plans/source-prism-platform.md`.
- No product scaffold files are created by the planner.
- `.omo/plans/source-prism-platform.md` includes scope, verification strategy, waves, dependency matrix, todos with exact QA invocations, roadmap-only guardrails, commit strategy, and success criteria.
