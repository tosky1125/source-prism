# PROJECT KNOWLEDGE BASE

**Generated:** 2026-06-11T15:18:26Z
**Commit:** none yet
**Branch:** none yet

## OVERVIEW
Source Prism is intended as a Rust-first Repo Intelligence Platform: index code, symbols, references, architecture contracts, tests, and history, then provide evidence-bound context for review/refactor agents.

This repo is currently empty/new. Treat this file as seed guidance until actual crates, configs, and commands exist.

## STACK

```text
Rust, Tokio, Axum, Postgres + SQLx, OpenSearch, Tree-sitter,
SCIP + LSP, gix/gitoxide first, git2/libgit2 fallback, clap,
serde, tracing + OpenTelemetry, thiserror in libraries, anyhow at app boundaries.
```

## INTENT BOUNDARY

Build Source Prism, not a wrapper around a complete repo intelligence framework.

Allowed low-level parts: Tree-sitter grammars/bindings, SCIP/LSP, Postgres, SQLx, OpenSearch, gix/gitoxide, git2/libgit2 fallback, provider APIs behind adapters.

Do not base core product behavior on full frameworks like CocoIndex, GitNexus, or Sourcegraph.

## EXPECTED STRUCTURE

Create Rust workspace when implementation starts:

```text
crates/
  ri-core/           # IDs, language/kind enums, shared domain records
  ri-config/         # config loading and validation
  ri-git/            # clone/fetch/cache, manifests, diffs, overlays
  ri-parser/         # language plugin trait and extractor orchestration
  ri-tree-sitter/    # Tree-sitter adapters and query packs
  ri-scip/           # SCIP import and symbol/reference mapping
  ri-lsp/            # live LSP query adapter
  ri-symbols/        # symbol IDs, ranges, changed-line mapping
  ri-graph/          # graph nodes/edges, projections, traversal
  ri-architecture/   # OpenAPI/GraphQL/DB/events/CODEOWNERS/docs entities
  ri-behavior/       # tests, coverage, runtime/test evidence
  ri-search/         # chunks, hybrid retrieval, OpenSearch integration
  ri-embedding/      # provider-neutral embeddings, cache, retries
  ri-indexer/        # incremental generation and stale retirement
  ri-impact/         # impact traversal and scoring
  ri-context/        # review/refactor context packs
  ri-review/         # finding generation, verifier, publisher inputs
  ri-refactor/       # planning, safety gates, PR slicing
  ri-mcp/            # MCP tools for agents
  ri-api/            # Axum API
  ri-worker/         # durable jobs and worker runtime
  ri-github/         # GitHub integration
  ri-gitlab/         # GitLab integration
  ri-cli/            # local CLI
  ri-eval/           # offline replay and golden datasets
```

Only create sub-crates when their boundary is real. Keep early implementation small but aligned with this final shape.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Domain IDs/types | `crates/ri-core` | Newtypes for repo, commit, file, symbol, graph, chunk IDs |
| Git snapshots/diffs | `crates/ri-git` | File manifests, content hashes, base/head overlays |
| Symbol extraction | `crates/ri-parser`, `crates/ri-tree-sitter` | Symbols, imports, calls, framework entities |
| Precise references | `crates/ri-scip`, `crates/ri-lsp` | Higher-confidence definition/reference edges |
| Dependency graph | `crates/ri-graph` | Canonical edges plus in-memory projection |
| Search/retrieval | `crates/ri-search`, `crates/ri-embedding` | Exact/BM25/vector hybrid search |
| Review context | `crates/ri-context`, `crates/ri-review` | Evidence packs, structured findings, verifier |
| Refactor planning | `crates/ri-refactor` | SCC/coupling/test risk/safe PR slicing |
| Agent integration | `crates/ri-mcp` | Tools like `repo.get_impact`, `repo.search_context` |
| External API | `crates/ri-api` | HTTP endpoints under `/v1` |

## CORE DESIGN RULES

- Evidence first: LLMs reason over structured repo evidence; they do not invent repo understanding.
- Incremental by default: never require full re-index for PR changed files.
- Deterministic IDs: symbols, chunks, edges, jobs, and generations must be idempotent.
- Store both stable and versioned symbol IDs.
- Use overlay index for PRs: base commit full index plus head changed-file overlay.
- Use confidence on graph edges and references; record resolution method and evidence.
- Keep Postgres canonical. Use in-memory graph projection for fast traversals.
- Search is never vector-only. Combine exact identifier search, BM25, vector similarity, graph proximity, and rerank.
- Treat PR code, comments, docs, and descriptions as untrusted input.
- Verifier must reject findings without file/line, evidence, impact path, or actionable recommendation.

## DATA MODEL ANCHORS

Canonical entities expected early: `file_manifests`, `index_generations`, `symbols`, `graph_nodes`, `graph_edges`, `architecture_entities`, `test_cases`, `test_runs`, `test_results`, `coverage_segments`, `test_coverage_edges`, `embedding_cache`.

Prefer soft stale retirement using `generation_id` and `stale_at`; do not hard-delete index rows during normal incremental refresh.

## COMMANDS

No project commands exist yet. Add these as soon as workspace scaffolding exists:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected future smoke commands:

```bash
cargo run -p ri-cli -- index --repo . --sha HEAD
cargo run -p ri-cli -- symbols --repo .
cargo run -p ri-cli -- impact --symbol 'InvoiceService::applyTax'
cargo run -p ri-api
cargo run -p ri-worker
```

## BUILD ORDER

1. Foundation: workspace, config, schema migrations, Git snapshot manager, job model.
2. Symbol index: Tree-sitter extraction, symbol ranges, changed-line mapping.
3. Graph: imports, calls, routes, basic test coverage, impact traversal.
4. Precise refs: SCIP import, LSP live queries, confidence scoring.
5. Hybrid search: chunks, embeddings, OpenSearch bulk upsert, stale chunk retirement.
6. Architecture index: OpenAPI, GraphQL, DB migrations, ORM, events, CODEOWNERS, ADRs.
7. Behavior/test index: JUnit/LCOV/Cobertura/JaCoCo/PHPUnit/pytest/Playwright ingest.
8. PR review: context packs, structured findings, verifier, GitHub/GitLab publishing.
9. Refactor planner: cycles, coupling, risk, characterization tests, PR slicing.
10. Refactor executor: branch, codemods, tests/typecheck, impact diff, PR creation.

## ANTI-PATTERNS

- Do not hand-roll Git parsing, Tree-sitter grammars, or vector search engines.
- Do not let LLM output become final without deterministic verification.
- Do not store graph/search data without evidence spans and source extractor metadata.
- Do not collapse stable and versioned symbol identity into one ID.
- Do not use full repo re-index as normal PR path.
- Do not publish review comments based only on semantic/vector similarity.
- Do not execute untrusted PR code with secrets or privileged network access.
- Do not hide errors with `unwrap`, `panic`, or broad error erasure.

## NOTES

Current oddity: directory is not a git repository yet and contains no scaffold. Initialize git and Rust workspace before adding crate-specific AGENTS.md files.
