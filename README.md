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

## Foundation Milestone

This repository is currently in the foundation phase. The first milestone creates the runnable base for later indexing work:

- Git repository hygiene and project documentation
- Rust workspace scaffold
- Core typed domain contracts
- Local Postgres/OpenSearch development stack
- SQLx migrations and offline query checks
- CLI, API, and worker smoke surfaces
- Evidence-based QA conventions

The foundation milestone deliberately does not implement full symbol extraction, graph impact analysis, search ranking, MCP tools, GitHub/GitLab publishing, PR review generation, or refactor execution. Those come after the base is executable and verified.

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

