# Source Prism

Source Prism turns a repository into queryable structure.

It indexes files, symbols, references, call graphs, architecture entities, test evidence, coverage, and searchable context so humans and AI agents can ask precise questions about code. Source Prism is not an LLM reviewer. It is the evidence layer that review, refactor, MCP, and repository-explorer tools can use without inventing repo knowledge.

## Why Source Prism?

Large codebases are hard to reason about because useful facts are scattered across syntax trees, tests, docs, migrations, API contracts, and Git history. Source Prism stores those facts in a deterministic model:

- stable and versioned symbol identities
- file manifests with generated, test, and vendor classification
- Tree-sitter symbols and call references
- graph edges for contains, imports, calls, and test coverage
- architecture entities from docs and contracts
- test and coverage evidence
- hybrid retrieval over identifiers, lexical matches, search chunks, and graph proximity
- API, CLI, Web UI, and MCP surfaces over the same indexed repo evidence

The goal is simple: agents and developers should reason from evidence, not guesses.

## Status

Source Prism is early but usable for local repository exploration.

Working today:

- Rust workspace with CLI, API, worker, web UI, and MCP crates
- Postgres canonical store and OpenSearch search sync
- local repository indexing with generation-based stale retirement
- symbol extraction for Rust, TypeScript/JavaScript/TSX, Python, and Go
- call graph and reference queries for direct identifier calls
- repository overview, files, symbols, references, impact, tests, runs, and search endpoints
- React repository explorer served by `ri-api`
- MCP tools for `repo.get_symbol`, `repo.find_references`, `repo.get_impact`, `repo.search_context`, and test context
- review finding verification and GitHub/GitLab dry-run payloads
- refactor planning evidence with execution disabled by design

Still maturing:

- precise cross-file references through SCIP/LSP
- richer framework and runtime edges
- authenticated multi-tenant API mode
- publishing adapters for real GitHub/GitLab review comments
- sandboxed execution gates for untrusted code

Source execution is intentionally disabled until sandboxing is designed.

## Supported Languages

Current parser support:

| Language | Symbols | Calls | Notes |
| --- | --- | --- | --- |
| Rust | yes | yes | functions, methods, modules, tests |
| TypeScript / TSX | yes | yes | identifier calls; receiver/member calls wait for precise resolution |
| JavaScript / JSX | yes | yes | same call-resolution limits as TypeScript |
| Python | yes | partial | symbols first, richer calls later |
| Go | yes | partial | functions, methods, package symbols |

Vendor trees such as `node_modules`, `vendor`, and `third_party` are excluded from indexing.

## Quick Start

Requirements:

- Rust stable
- Docker with Compose
- `cargo sqlx` for SQLx metadata work
- Bun if you want to build the web UI from `apps/web`

Start dependencies:

```bash
docker compose up -d postgres opensearch
```

Load environment:

```bash
export DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism
export OPENSEARCH_URL=http://localhost:9200
export API_BIND_ADDR=127.0.0.1:3000
```

Run migrations:

```bash
cargo run -p ri-cli -- db migrate
```

Index this repository:

```bash
cargo run -p ri-cli -- index --repo . --sha HEAD
```

Start the API and Web UI:

```bash
cargo run -p ri-api
```

Open:

```text
http://127.0.0.1:3000/repo/source-prism
```

Sync search chunks to OpenSearch after an index run:

```bash
cargo run -p ri-worker -- --once
```

## CLI Examples

Inspect a repository manifest:

```bash
cargo run -p ri-cli -- repo manifest --repo .
```

List extracted symbols:

```bash
cargo run -p ri-cli -- symbols --repo .
```

Find references for a symbol:

```bash
cargo run -p ri-cli -- references --repo . --symbol App::runSearch
```

Analyze impact:

```bash
cargo run -p ri-cli -- impact --repo . --symbol search_context_command
```

Build a context pack:

```bash
cargo run -p ri-cli -- search-context --repo . search_context
```

Get related tests:

```bash
cargo run -p ri-cli -- test-context --repo . --symbol extracts_rust_functions_methods_and_tests
```

List MCP tools:

```bash
cargo run -p ri-cli -- mcp tools
```

Call an MCP tool once:

```bash
cargo run -p ri-cli -- mcp call --repo . --tool repo.get_symbol --symbol search_context_command
```

## API Examples

Health:

```bash
curl -fsS http://127.0.0.1:3000/v1/health
```

Register a repo:

```bash
curl -fsS -X POST http://127.0.0.1:3000/v1/repos \
  -H 'content-type: application/json' \
  --data '{"repo_id":"source-prism","name":"source-prism","default_branch":"main"}'
```

Index a repo:

```bash
curl -fsS -X POST http://127.0.0.1:3000/v1/repos/source-prism/index \
  -H 'content-type: application/json' \
  --data '{"sha":"HEAD","repo_path":"."}'
```

Query references:

```bash
curl -fsS 'http://127.0.0.1:3000/v1/repos/source-prism/references?symbol=App::runSearch'
```

Search context:

```bash
curl -fsS -X POST http://127.0.0.1:3000/v1/repos/source-prism/context/search \
  -H 'content-type: application/json' \
  --data '{"query":"search_context","limit":5}'
```

## Web UI

The web UI is a React app in `apps/web` and is served by `ri-api` from:

```text
/repo/:repo_id
/repo/:repo_id/files
/repo/:repo_id/symbols
/repo/:repo_id/impact
/repo/:repo_id/search
```

Build it with:

```bash
cd apps/web
bun install
bun run check
bun run build
```

The build output is copied into `crates/ri-api/assets/repo-explorer`.

## Architecture

Source Prism is split into small Rust crates:

| Crate | Role |
| --- | --- |
| `ri-core` | IDs, language/kind enums, shared domain records |
| `ri-git` | worktree discovery, manifests, path classification |
| `ri-parser` | parser traits and source-file boundary |
| `ri-tree-sitter` | Tree-sitter symbol and call extraction |
| `ri-symbols` | symbol records, ranges, changed-line mapping |
| `ri-graph` | graph nodes and edges |
| `ri-indexer` | generation indexing, stale retirement, search chunks |
| `ri-impact` | impact traversal and scoring |
| `ri-context` | evidence-backed context packs and references |
| `ri-search` | hybrid retrieval primitives |
| `ri-behavior` | test and coverage ingestion |
| `ri-architecture` | docs and contract entity extraction |
| `ri-review` | finding verification and publisher payload inputs |
| `ri-refactor` | planner-only refactor evidence |
| `ri-mcp` | MCP tool contracts and handler |
| `ri-api` | Axum API and web shell |
| `ri-worker` | durable background job processing |
| `ri-cli` | local command-line interface |

Postgres is canonical. OpenSearch is a derived search projection. In-memory graph projections are used only as fast views over indexed evidence.

## Development

Core checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

SQLx metadata:

```bash
cargo sqlx migrate run
cargo sqlx prepare --workspace --check
```

Web checks:

```bash
cd apps/web
bun run check
bun run build
```

CI runs the Rust checks, SQLx checks, CLI smoke commands, API smoke commands, worker search sync, and web asset checks.

## Security Model

Source Prism treats repository code, PR text, comments, and docs as untrusted input.

Current rules:

- do not execute indexed source code
- do not let LLM output become final without deterministic verification
- do not publish findings without file/line, evidence, impact path, and actionable recommendation
- do not use vector-only retrieval for review or refactor evidence
- keep stable and versioned symbol identity separate
- prefer incremental overlays for PR workflows instead of full re-index as the normal path

Network binding is local by default. Public bind addresses are rejected unless auth and tenancy gates are explicitly configured.

## Contributing

Issues and PRs are welcome.

Good first contribution areas:

- parser fixtures for supported languages
- additional framework/entity extractors
- better graph edge confidence and evidence spans
- docs and examples for real repositories
- UI improvements for repository exploration
- tests for CLI/API behavior that currently lacks real-surface coverage

Before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If you touch `apps/web`, also run:

```bash
cd apps/web
bun run check
bun run build
```

## Roadmap

Near-term:

- precise SCIP/LSP references
- richer TypeScript/Python/Go call resolution
- repo explorer graph interactions
- durable MCP server mode
- search drift repair UX

Later:

- PR overlay indexing by base/head commit
- GitHub/GitLab publishing adapters
- sandboxed execution and test gates
- refactor executor with branch safety
- offline evaluation datasets

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
