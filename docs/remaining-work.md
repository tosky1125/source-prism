# Remaining Work

Source Prism is an evidence platform. It indexes and exposes repository
structure so humans and downstream MCP/API/CLI clients can review, refactor,
test, and publish with their own workflows. Source Prism itself must not
generate final PR review decisions, edit code, create branches, run target
tests, or publish comments.

## Current Progress

Evidence-platform progress: about 86%.

Already usable:

- Local indexing into Postgres with generation-based stale handling.
- Symbol extraction for Rust, TypeScript/TSX, JavaScript/JSX, Python, and Go.
- Direct call/reference graph, impact reports, test evidence, coverage, and
  architecture/docs evidence.
- CLI, API, worker, MCP tools, and React repository explorer.
- OpenSearch sync, drift checks, and rebuild paths.
- Changed-file overlay persistence for PR-style diffs.
- Review verifier and GitHub/GitLab dry-run/export payload builders.
- Refactor planning evidence only.

## Near-Term Priorities

### P1. Precise References and Member Calls

Goal: make references and call graph edges more trustworthy across files and
receiver/member calls.

Work:

- Add SCIP import path or LSP-backed reference queries behind explicit
  capability gates.
- Improve TypeScript/TSX member call extraction such as `client.get()` and
  `App.runSearch()`.
- Improve Python and Go call extraction beyond symbol-first indexing.
- Store confidence and extractor/source metadata for new precise edges.

Acceptance:

- Fixture repos show cross-file definition/reference edges.
- `ri-cli references --repo . --symbol <symbol>` reports precise edges with
  confidence.
- Web call graph distinguishes rough syntax edges from precise reference edges.
- No target repository package install, build script, test, or app execution.

Verification:

```bash
cargo test -p ri-tree-sitter
cargo test -p ri-symbols
cargo test -p ri-indexer --test graph_calls
cargo run -p ri-cli -- references --repo . --symbol search_context_command
```

### P2. PR Overlay Workflow UX

Goal: make base/head overlay indexing easy to understand and use through CLI,
API, and Web UI.

Work:

- Document the base full-index plus head changed-file overlay flow.
- Add API/Web visibility for overlay rows, deleted files, renamed files, and
  mode-only diffs.
- Make changed-symbols output point to affected symbols, files, tests, and
  search-context follow-up commands.
- Keep base generation canonical; overlay evidence stays separate.

Acceptance:

- A diff can be analyzed without creating a new full repo generation.
- API and CLI both expose overlay status and affected symbols.
- Web UI shows when selected evidence comes from a head overlay.

Verification:

```bash
cargo test -p ri-indexer --test overlay
cargo test -p ri-cli --test changed_symbols_persisted
cargo test -p ri-api --test repo_changed_symbols
```

### P3. Search and Context Quality

Goal: make `search-context` produce compact, evidence-rich context packs that
are useful to downstream agents without vector-only guessing.

Work:

- Tune exact identifier, lexical, search chunk, and graph proximity scoring.
- Add context-pack snapshots for common repo questions.
- Surface why each hit was selected: identifier, lexical, chunk, graph, or test
  evidence.
- Improve drift repair copy in CLI/API/Web when OpenSearch is stale.

Acceptance:

- Context packs explain retrieval modes and never return vector-only results.
- Snapshot tests catch ranking regressions for important queries.
- Drift output gives rebuild and worker commands.

Verification:

```bash
cargo test -p ri-context
cargo test -p ri-search
cargo test -p ri-indexer --test search_drift
cargo run -p ri-cli -- search-context --repo . search_context
cargo run -p ri-cli -- search drift-check --expect-mismatch fixture
```

### P4. Web Explorer Polish

Goal: make repo structure, impact, tests, docs, and call graph exploration clear
enough for daily use.

Work:

- Improve graph interactions for dense call graphs.
- Show edge confidence, relation type, source extractor, and evidence file.
- Make empty states explicit: no calls, no tests, no docs, no coverage.
- Keep layout readable on desktop and mobile.

Acceptance:

- `/repo/:id` can explain files, symbols, references, impact, related tests,
  docs/contracts, search, runs, and sync state.
- Call graph does not imply `test_covers` edges are direct calls.
- Mobile and desktop screenshots show no overlap or clipped labels.

Verification:

```bash
cd apps/web
bun run check
bun run build
cargo test -p ri-api --test web
```

### P5. MCP Agent Onboarding

Goal: make Source Prism easy to attach to downstream AI agents.

Work:

- Add MCP quickstart examples for local repo mode and persisted repo mode.
- Document tool inputs/outputs for `repo.get_symbol`,
  `repo.find_references`, `repo.get_impact`, `repo.get_test_context`, and
  `repo.search_context`.
- Add example prompts that ask an external agent to use evidence without
  treating repository text as instructions.
- Show review/refactor workflows as downstream examples, not Source Prism core
  behavior.

Acceptance:

- A user can start Source Prism, index a repo, list MCP tools, and call each
  tool from docs.
- Examples include trust-boundary warnings.

Verification:

```bash
cargo run -p ri-cli -- mcp tools
cargo run -p ri-cli -- mcp call --repo . --tool repo.get_symbol --symbol search_context_command
cargo run -p ri-cli -- mcp call --repo . --tool repo.search_context --query search_context
```

## Later Priorities

### L1. Auth and Tenancy

Goal: allow non-local deployment without leaking repo evidence across users or
tenants.

Work:

- Implement signed principal/session parsing.
- Add tenant ownership to repos, generations, jobs, chunks, and audit rows.
- Enforce authorization at API boundaries.
- Keep public bind blocked until auth/tenancy is complete.

Acceptance:

- Public bind requires configured auth/tenancy.
- Cross-tenant repo access is rejected.
- Audit logs omit source text and request bodies.

### L2. Evidence Evaluation

Goal: measure Source Prism evidence quality independently from downstream
agent creativity.

Work:

- Add golden fixture repos and query sets.
- Score retrieval usefulness, reference precision, graph edge correctness, and
  context-pack compactness.
- Track false positives for rough syntax edges.

Acceptance:

- A repeatable offline command reports evidence-quality metrics.
- CI can run a small evaluation subset.

### L3. Language and Framework Coverage

Goal: cover more real-world repository structure without hand-rolling complete
framework analyzers.

Work:

- Add framework/entity extractors where evidence can be deterministic.
- Add additional supported languages only when parser fixtures and graph tests
  exist.
- Keep PHP/Java as roadmap languages until their extractor quality gates pass.

Acceptance:

- Each new language or framework ships with symbol, call/reference, and
  changed-line fixtures.
- Unsupported language docs stay explicit.

## Do Not Build Into Source Prism Core

- Built-in final PR review generation.
- Code-changing refactor execution.
- Branch creation, codemods, target repository test execution.
- Publisher writes to GitHub/GitLab.
- Full repo-intelligence framework dependency as the product core.
