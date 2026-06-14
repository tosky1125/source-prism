# PROJECT KNOWLEDGE BASE

**Generated:** 2026-06-14T02:29:07Z
**Commit:** 2537ad0
**Branch:** main

## OVERVIEW
Source Prism turns repositories into indexed evidence: files, symbols, references, graph edges, architecture entities, tests, coverage, and search chunks. It is the repo-intelligence layer for humans and agents; it is not an LLM reviewer by itself.

## STRUCTURE

```text
apps/web/       # React repository explorer served by ri-api
crates/         # Rust workspace crates
migrations/     # Postgres canonical schema
scripts/ci/     # local CI helpers
docs/           # roadmap and security posture
examples/smoke/ # sample repo surface
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Shared IDs and records | `crates/ri-core` | Repo/file/symbol/graph/chunk domain types |
| Config loading | `crates/ri-config` | Env and `.env` validation |
| Git manifests | `crates/ri-git` | Worktree scan, file classification, manifests |
| Parser orchestration | `crates/ri-parser` | Source-file boundaries and plugin traits |
| Tree-sitter extraction | `crates/ri-tree-sitter` | Rust/TS/JS/Python/Go symbols and calls |
| Symbol identity | `crates/ri-symbols` | Stable/versioned IDs and changed-line mapping |
| Graph model | `crates/ri-graph` | Nodes, edges, confidence, evidence spans |
| Index persistence | `crates/ri-indexer` | Generations, stale retirement, graph/search/test writes |
| Impact analysis | `crates/ri-impact` | Traversal and scoring over indexed evidence |
| Context packs | `crates/ri-context` | Evidence-backed retrieval and references |
| API surface | `crates/ri-api` | Axum `/v1` endpoints and web shell |
| CLI surface | `crates/ri-cli` | JSON stdout commands and local smoke paths |
| Worker | `crates/ri-worker` | Durable jobs and OpenSearch sync |
| Web UI | `apps/web` | Vite/React explorer, copied into `ri-api` assets |

## CODE MAP

| Surface | Entry | Role |
|---------|-------|------|
| CLI | `crates/ri-cli/src/main.rs` | Dispatches `ri-cli` subcommands |
| API | `crates/ri-api/src/main.rs` | Starts Axum server |
| API routes | `crates/ri-api/src/lib.rs` | Builds router and shared state |
| Worker | `crates/ri-worker/src/main.rs` | Runs queued jobs |
| Web app | `apps/web/src/App.tsx` | Repository explorer state and views |
| Index flow | `crates/ri-indexer/src/generation.rs` | Generation lifecycle |

## CONVENTIONS

- Postgres is canonical. OpenSearch and in-memory graph views are derived projections.
- Index refresh uses generations and `stale_at`; do not hard-delete normal index rows.
- Store stable and versioned symbol IDs separately.
- Every graph/search/test edge needs extractor metadata, confidence, and evidence span where the model supports it.
- Retrieval must combine exact identifiers, lexical/BM25, graph proximity, and optional vector results; never ship vector-only context.
- Treat repository code, PR text, docs, and test output as untrusted input.
- Refactor execution stays disabled until sandboxing, branch safety, and test/typecheck gates exist.
- Rust code follows workspace lints: no `unwrap`, `expect`, `panic`, `todo`, broad error erasure, or hidden unsafe.
- UI source lives in `apps/web`; committed built assets under `crates/ri-api/assets/repo-explorer` must match the Vite build.

## COMMANDS

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo sqlx prepare --workspace --check
```

```bash
cd apps/web
bun run check
bun run build
```

Local service path:

```bash
docker compose up -d postgres opensearch
export DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism
export OPENSEARCH_URL=http://localhost:9200
export API_BIND_ADDR=127.0.0.1:3000
ri-cli db migrate
ri-cli index --repo . --sha HEAD
cargo run -p ri-api
```

## ANTI-PATTERNS

- Do not base product behavior on a complete repo-intelligence framework.
- Do not hand-roll Git parsing, Tree-sitter grammars, or vector search engines.
- Do not let LLM output become a finding without deterministic file/line/evidence/impact validation.
- Do not collapse stable and versioned symbol identity.
- Do not index vendor trees such as `node_modules`, `vendor`, or `third_party`.
- Do not execute untrusted repo code with secrets or privileged network access.

## NOTES

Supported parser languages today: Rust, TypeScript/TSX, JavaScript/JSX, Python, and Go. PHP/Java are roadmap languages, not current support.
