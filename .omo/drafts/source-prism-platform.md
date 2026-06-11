# Source Prism Platform Planning Draft

## Request

Plan a Rust-first Repo Intelligence Platform.

User constraints:

- Use Rust if preferred; single-language Rust path is acceptable.
- Do not use complete repo-intelligence frameworks like CocoIndex, GitNexus, or Sourcegraph.
- Do use low-level components: Tree-sitter, SCIP/LSP, OpenSearch, Postgres, gix/gitoxide/git2 fallback.
- Product goal: repo intelligence, not hand-written Git/parser/vector engines.
- Planner must ask before assuming strange or ambiguous choices.

## Skill / Mode

- `omo:ulw-plan`: active. Planner-only. Explore first, ask unresolved decisions, wait for explicit approval before `.omo/plans`.
- `caveman`: active. Terse output only; technical detail preserved.
- `omo:programming`: not active for edits yet. No `.rs` implementation in this planning turn.

## Tier

HEAVY.

Reason: architecture-scale bootstrap with new workspace, many crates/modules, DB schema, search, external integrations, worker jobs, sandbox/security, incremental invalidation, and caching.

## Current Repo Facts

Evidence:

- `AGENTS.md` exists and governs repo.
- `.omo/drafts/` exists from this planning run.
- No `.git/`.
- No `Cargo.toml`.
- No `crates/`.
- No migrations.
- No Docker/Compose.
- No CI.
- No tests.
- `git status --short --branch` fails: `fatal: not a git repository (or any of the parent directories): .git`.

Implication: first implementation plan must start with repo/bootstrap foundation, not feature modules.

## Seed Architecture Facts From AGENTS.md

- Target stack: Rust, Tokio, Axum, Postgres + SQLx, OpenSearch, Tree-sitter, SCIP + LSP, gix/gitoxide first, git2 fallback, clap, serde, tracing/OpenTelemetry.
- Expected final crates include `ri-core`, `ri-config`, `ri-git`, `ri-parser`, `ri-tree-sitter`, `ri-scip`, `ri-lsp`, `ri-symbols`, `ri-graph`, `ri-architecture`, `ri-behavior`, `ri-search`, `ri-embedding`, `ri-indexer`, `ri-impact`, `ri-context`, `ri-review`, `ri-refactor`, `ri-mcp`, `ri-api`, `ri-worker`, `ri-github`, `ri-gitlab`, `ri-cli`, `ri-eval`.
- Early entities: `file_manifests`, `index_generations`, `symbols`, `graph_nodes`, `graph_edges`, `architecture_entities`, `test_cases`, `test_runs`, `test_results`, `coverage_segments`, `test_coverage_edges`, `embedding_cache`.
- Core rules: evidence-first, incremental indexing, deterministic IDs, stable + versioned symbol IDs, PR overlay index, confidence on edges, Postgres canonical, OpenSearch secondary, vector search never alone, untrusted PR text/code/docs.

## External Docs Checked

- Axum Context7 `/tokio-rs/axum`: current patterns use `Router`, `with_state`, `State<T>`, `tower::ServiceBuilder`, Tower middleware, `axum::serve`.
- SQLx Context7 `/websites/rs_sqlx`: `query!` macros are statically checked; offline mode uses `cargo sqlx prepare` and checked-in `.sqlx`; transactions roll back on drop if not committed.
- OpenSearch docs/source: official Rust client exists for indexing/querying; OpenSearch Bulk API reduces network round trips for indexing/update/delete batches.
- Tree-sitter docs/source: Rust bindings parse source with language grammars; Tree-sitter is parser generator + incremental parsing library.
- gix/gitoxide docs/source: Rust Git implementation/library oriented toward correctness/performance.
- MCP docs/source: MCP servers expose tools with names and schemas; this must be treated as an integration surface and security boundary.

## Verified Planning Inputs

Repo surface lane:

- Only `AGENTS.md` and `.omo/` exist.
- Plan must initialize git/Rust workspace before crate work.

Package/test lane:

- No package manager/build/test/CI surface.
- Need baseline commands: `cargo fmt`, `cargo clippy`, `cargo test`.
- Need Docker/Compose or equivalent dev stack for Postgres/OpenSearch.
- Need migration scaffold and test conventions.

Execution workflow lane:

- First milestone should be runnable skeleton:
  - git repo
  - Rust workspace
  - local DB/search dev stack
  - migrations
  - CLI/API/worker entrypoints
  - evidence layout

Risk/QA lane:

- No contradictions found.
- Critical gaps:
  - sandbox scope and isolation mechanism
  - prompt/context trust boundary
  - Postgres/OpenSearch consistency model
  - incremental invalidation and overlay semantics
  - resolver confidence thresholds
  - eval metrics
  - auth/tenancy/token handling
  - worker retry/dead-letter/locking
  - data retention and stale compaction

## Recommended Defaults For User Decisions

1. Plan scope default: first executable foundation milestone, not full product implementation.
   Rationale: repo has no scaffold; full system plan can be long-term roadmap but worker needs startable tasks.

2. Test strategy default: TDD for domain invariants and schema/overlay logic, tests-after for thin CLI/API smoke wiring, real-surface QA for CLI/API/worker.
   Rationale: behavior-heavy index invariants need RED/GREEN; bootstrap wiring needs executable smoke proof.

3. Local infra default: Docker Compose for Postgres + OpenSearch first.
   Rationale: makes SQLx migrations, OpenSearch indexing, and CI parity concrete.

4. Consistency default: Postgres canonical; OpenSearch eventually consistent via idempotent jobs with drift repair.
   Rationale: avoids dual-write atomicity trap while keeping rebuild path.

5. Sandbox default: parse/index path is read-only/no-network; any code execution/test/refactor path runs in separate sandbox with no secrets, restricted network, CPU/memory/time limits.
   Rationale: user security constraints plus untrusted PR code.

6. Confidence default: typed confidence tiers (`Exact`, `High`, `Medium`, `Low`) with method/evidence; review publishing requires `High` or explicit verifier-backed evidence.
   Rationale: consumers need deterministic filtering.

## Open Questions Before Approval Brief

Planner lane recommends future wave order:

1. Repo foundation: git init, Rust workspace, minimal crates, lint/toolchain, CI commands.
2. Core domain + config: IDs/types, evidence spans, confidence model, deterministic IDs.
3. Persistence foundation: Postgres schema, SQLx migrations, generations/stale retirement, jobs.
4. Git snapshot + overlay model: gix-first manifests, hashes, base/head overlay, git2 boundary.
5. Symbol extraction MVP: Tree-sitter, symbol ranges, imports, changed-line mapping.
6. Graph + impact traversal: nodes/edges, confidence/evidence metadata, in-memory projection.
7. Search + chunks: OpenSearch secondary index, chunks, hybrid retrieval, stale chunks.
8. API/CLI/MCP surfaces: index/symbol/search/impact/context tools.
9. Precise refs + behavior evidence: SCIP, LSP, coverage/test ingest.
10. Review/refactor context layer: context packs, verifier, PR slicing inputs.

Questions to ask now:

1. Plan deliverable scope:
   - Recommended: one executable foundation milestone plus explicit later roadmap.
   - Alternative: full 10-wave master plan now.
   - Alternative: only high-level architecture plan, no executable task breakdown.

2. Test strategy:
   - Recommended: TDD for domain/schema/overlay invariants, tests-after for thin CLI/API smoke wiring, real-surface QA for CLI/API/worker.
   - Alternative: tests-after everywhere.
   - Alternative: no automated tests in first scaffold, QA smoke only.

3. Implementation bias:
   - Recommended: small initial crates (`ri-core`, `ri-config`, `ri-git`, `ri-indexer`, `ri-cli`, `ri-api`, `ri-worker`) and add specialized crates when boundaries become real.
   - Alternative: create all final crates up front as empty shells.
   - Alternative: start with fewer crates (`ri-core`, `ri-git`, `ri-cli`) and postpone API/worker/infra.
