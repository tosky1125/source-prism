# RI-API KNOWLEDGE

## OVERVIEW
`ri-api` is the Axum HTTP surface for repo indexing, search, graph, runs, tests, review/refactor evidence, and the web explorer shell.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Server entry | `src/main.rs` | Runtime startup and config |
| Router/state | `src/lib.rs`, `src/state.rs` | App wiring |
| Error contract | `src/error.rs` | JSON errors |
| Local repo runs | `src/repos/local.rs`, `src/run_jobs.rs` | Job-backed local indexing |
| Index endpoints | `src/repo_index.rs`, `src/repo_index_jobs.rs` | POST index and job evidence |
| Repo data endpoints | `src/repo_files.rs`, `src/repo_symbols.rs`, `src/repo_references.rs` | Explorer data |
| Web shell | `src/web.rs` | Static assets and repo routes |
| Integration tests | `tests/` | Real endpoint behavior |

## CONVENTIONS

- Keep `/v1` JSON shapes stable; CLI and web depend on them.
- Prefer typed response structs over ad hoc JSON maps.
- Database-backed tests use Postgres fixtures; no silent fallback when persistence is required.
- `ri-api` serves built web assets from `assets/repo-explorer`; source is `apps/web`.
- Long-running index work should be modeled as jobs/runs, not hidden request side effects.
- Preserve evidence counts on run responses: files, symbols, graph edges, search chunks, tests, docs.

## COMMANDS

```bash
cargo test -p ri-api
cargo test -p ri-api --test web
```

With services configured:

```bash
cargo run -p ri-api
curl -fsS http://127.0.0.1:3000/v1/health
```

## ANTI-PATTERNS

- Do not return HTTP 200 with empty or ambiguous error payloads.
- Do not make web routes bypass the same indexed evidence used by API endpoints.
- Do not hard-delete generation rows referenced by jobs; use cascade-aware migrations or soft stale handling.
