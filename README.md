# Source Prism

Source Prism is a Rust-first Repo Intelligence Platform.

Its core job is to turn repositories into queryable structure: file manifests, symbols, references, dependency edges, architecture entities, test evidence, historical signals, and searchable context. PR review, refactoring help, MCP tools, and UI exploration are downstream products powered by that structured repository model.

## Product Direction

Source Prism is not a wrapper around complete repo-intelligence products such as CocoIndex, GitNexus, or Sourcegraph. It uses low-level components as building blocks:

- Tree-sitter for syntax parsing and symbol extraction
- SCIP and LSP for precise references
- Postgres and SQLx for canonical storage
- OpenSearch for secondary keyword/vector/hybrid retrieval
- gix/gitoxide first for Git plumbing, with git2/libgit2 fallback if needed
- Tokio and Axum for service runtime and HTTP API

## Current Milestone

This repository is currently past the bare foundation phase. The runnable base now includes:

- Git repository hygiene and project documentation
- Rust workspace scaffold
- Core typed domain contracts
- Local Postgres/OpenSearch development stack
- SQLx migrations and offline query checks
- CLI, API, and worker smoke surfaces
- Tree-sitter symbol extraction for Rust, TypeScript/JavaScript, Python, and Go
- Postgres-backed file manifest, symbol, graph, test-case, run, and search outbox indexing
- Static test-context and `test_covers` graph evidence from extracted test symbols
- Evidence-based QA conventions

MCP tools, GitHub/GitLab publishing, PR review generation, and refactor execution are still roadmap work. Source execution remains forbidden until sandbox design lands.

## Local Smoke Commands

```bash
cargo run -p ri-cli -- config check --env-file .env.example
cargo run -p ri-cli -- db migrate
cargo run -p ri-cli -- repo manifest --repo .
cargo run -p ri-cli -- index --repo . --sha HEAD
cargo run -p ri-cli -- symbols --repo .
curl -fsS http://127.0.0.1:3000/v1/repos/source-prism-ci/tests
cargo run -p ri-cli -- impact --symbol search
cargo run -p ri-cli -- search-context search
cargo run -p ri-cli -- test-context --symbol extracts_rust_functions_methods_and_tests
cargo run -p ri-api
curl -fsS -X POST http://127.0.0.1:3000/v1/test-context \
  -H 'content-type: application/json' \
  --data '{"symbol":"extracts_rust_functions_methods_and_tests"}'
cargo run -p ri-worker -- --once
```

## Planning Artifacts

The current execution plan lives at:

```text
.omo/plans/source-prism-platform.md
```

Seed agent guidance lives at:

```text
AGENTS.md
```

## Expected Future Checks

These commands become authoritative once the Rust workspace exists:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## SQLx Offline Metadata

SQLx migrations are part of the foundation gate. After changing SQL
queries or migrations, update offline metadata against a migrated local
Postgres database:

```bash
docker compose up -d postgres
DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx migrate run
DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx prepare --workspace
```

CI checks the committed metadata with:

```bash
DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism cargo sqlx prepare --workspace --check
```

The current `.sqlx/.gitkeep` keeps the policy directory tracked until
the first compile-time checked SQLx query generates metadata files.

## Evidence And QA

Agent-run verification evidence is written under `.omo/evidence/` using this naming pattern:

```text
.omo/evidence/task-<N>-<slug>.<ext>
```

Evidence files are local run artifacts and are ignored by git, except for `.omo/evidence/.gitkeep`.

Work should keep RED/GREEN proof when practical:

- RED evidence captures the failing or missing behavior before implementation.
- GREEN evidence captures the exact command, API call, or real-surface workflow after implementation.
- Real-surface evidence is preferred over mocked success output for CLI, API, worker, database, and search paths.

Do not hide verification behind opaque scripts. Wrappers may exist, but task acceptance should still cite the exact command or endpoint that was exercised.
