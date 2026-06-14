# MIGRATIONS KNOWLEDGE

## OVERVIEW
`migrations` defines the canonical Postgres schema used by SQLx and all persisted repo evidence.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Foundation schema | `20260611182000_foundation_schema.sql` | Core repo/generation/file/symbol/graph tables |
| Test cases | `20260612130000_test_cases.sql` | Test symbol evidence |
| Architecture entities | `20260612150000_architecture_entities.sql` | Docs/contracts/entities |
| Test runs | `20260612170000_test_runs.sql` | Runs/results |
| Coverage | `20260612180000_coverage_segments.sql` | Coverage segments |
| Job metadata | `20260612190000_jobs_metadata.sql` | Queue/run links |
| Generation delete cascade | `20260612200000_jobs_generation_delete_cascade.sql` | FK behavior for jobs |
| File overlays | `20260612210000_file_overlays.sql` | PR/overlay indexing |

## CONVENTIONS

- Add forward-only migrations; do not edit applied migration files unless the project explicitly resets history.
- Preserve canonical evidence semantics: generated rows should keep repo, commit, generation, and stale state clear.
- Prefer constraints that protect indexed evidence from orphaning.
- When schema or query shape changes, refresh SQLx metadata.
- Migration tests should exercise FK behavior that previously broke CI.

## COMMANDS

```bash
cargo sqlx migrate run
cargo sqlx prepare --workspace --check
cargo test -p ri-api --test job_cascade
```

## ANTI-PATTERNS

- Do not add cascades casually; verify what evidence disappears.
- Do not rely on application code to enforce invariants that belong in FK/unique constraints.
- Do not make schema changes without checking both API and indexer tests.
