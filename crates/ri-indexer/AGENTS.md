# RI-INDEXER KNOWLEDGE

## OVERVIEW
`ri-indexer` persists repository evidence into Postgres and derived search projections.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Generation lifecycle | `src/generation.rs` | Run/generation creation and status |
| File manifests | `src/generation_files.rs`, `src/generation_manifest.rs` | File rows and stale handling |
| Symbols | `src/symbols.rs`, `src/symbols/` | Stable/versioned symbol persistence |
| Graph writes | `src/graph.rs`, `src/graph_calls.rs`, `src/graph_imports.rs` | Contains/imports/calls/test coverage |
| Search chunks | `src/search_chunks.rs`, `src/search_sync*.rs` | Postgres chunks and OpenSearch sync |
| Test evidence | `src/test_cases.rs`, `src/test_runs/` | Test cases, runs, results |
| Coverage | `src/coverage.rs` | Coverage segment ingestion |

## CONVENTIONS

- Normal refresh is generation-based and stale-aware; avoid destructive cleanup.
- Keep graph edges evidence-backed with confidence and extractor/source metadata.
- Search sync drift means Postgres expected count and OpenSearch actual count disagree; fix source or sync, not the assertion.
- Path classification must exclude generated/vendor trees before symbol/search indexing.
- Inserts should be idempotent for reruns of the same repo/commit.
- Keep SQLx query metadata current after changing SQL.

## COMMANDS

```bash
cargo test -p ri-indexer
cargo sqlx prepare --workspace --check
```

## ANTI-PATTERNS

- Do not make OpenSearch canonical.
- Do not delete `index_generations` without considering `jobs.generation_id`.
- Do not index `node_modules`, `vendor`, `third_party`, build output, or generated web assets as source evidence.
