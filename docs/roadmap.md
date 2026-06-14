# Roadmap Guardrails

Foundation is the platform skeleton: workspace, configuration, migrations, Git manifests, generation bookkeeping, durable jobs, local API health, search sync contracts, CLI command surface, CI, and security boundaries. Everything below is Roadmap only until its entry condition and evidence gate are satisfied.

## Rule

Roadmap only means the crate or feature may have a placeholder command, schema anchor, trait, or document, but it is not a milestone-1 implementation task. It must not be treated as production behavior until the required evidence exists.

## Post-Foundation Waves

Detailed active priorities and acceptance criteria live in
`docs/remaining-work.md`.

| Area | Crates | Entry condition | Required evidence | Why not created in foundation |
| --- | --- | --- | --- | --- |
| Symbol extraction | `ri-parser`, `ri-tree-sitter`, `ri-symbols` | Stable file manifest and generation lifecycle are available | Tree-sitter fixture suite, changed-line to innermost-symbol mapping, language coverage report | Foundation only proves repo read and manifest paths; parser correctness needs language fixtures |
| Dependency graph and impact | `ri-graph`, `ri-impact` | Symbol records and rough references exist | Edge fixtures with confidence and evidence spans, traversal tests, impact CLI smoke | Graph behavior without symbols would be fake structure |
| Hybrid search and chunks | `ri-search`, `ri-embedding` | Symbol/test/doc chunk plans exist and OpenSearch sync is stable | Exact identifier, BM25, vector, and drift-check fixtures with stale retirement proof | Foundation only proves sync contracts, not retrieval quality |
| SCIP and LSP references | `ri-scip`, `ri-lsp` | Rough symbol/reference layer exists | SCIP import fixtures, LSP timeout/failure tests, confidence calibration | Precise references need sandbox and language-specific behavior |
| Architecture evidence | `ri-architecture` | Parser and manifest paths can locate contracts | OpenAPI, GraphQL, DB migration, event, CODEOWNERS, ADR fixtures | Foundation cannot infer architecture without extractors |
| Behavior and tests | `ri-behavior` | Test artifacts are available without executing target code | JUnit, LCOV, Cobertura, JaCoCo, PHPUnit, pytest, Playwright ingestion fixtures | Foundation forbids target repository execution |
| MCP integration | `ri-mcp` | Query APIs return evidence-bound symbols, graph, search, and test context | MCP tool smoke, trust-boundary tests, no arbitrary shell execution proof | Agent tools need real query surfaces to expose |
| GitHub and GitLab dry-run exports | `ri-github`, `ri-gitlab` | Review findings have verifier-backed evidence | Check/comment dry-run fixtures, SARIF/code-quality output proof | Source Prism can shape payloads without becoming the publisher |
| Downstream agent evidence packs | `ri-context`, `ri-review`, `ri-refactor` | Impact, retrieval, architecture, and behavior evidence exist | Context pack snapshots, verifier rejection tests, MCP smoke | LLM reasoning and code edits happen in external clients, not Source Prism |
| Evaluation | `ri-eval` | Evidence packs and verifier outputs are stable | Golden dataset schema, offline replay, retrieval/usefulness metrics | Evaluation scores Source Prism evidence quality, not agent creativity |

## Future Crate Creation

Create a new crate only when its boundary has real code, tests, and observable behavior. A planned crate name alone is not enough.

Roadmap-only crates may appear in documentation and command placeholders. They should not be added to the workspace until at least one milestone story needs compiled Rust code in that boundary.

## Product Boundary

Source Prism is the evidence platform. It must not grow built-in PR review generation, code-changing refactor execution, branch creation, target test execution, or publisher writes as core behavior. Those workflows belong to downstream MCP/API/CLI clients that choose how to use Source Prism evidence.

## Command Guardrails

Future CLI commands may expose placeholders for roadmap-only behavior. Those commands must return machine-readable `not_implemented` status and a non-zero exit code until the roadmap entry condition is met.

Commands documented in `README.md` are expected to drive real CLI/API/MCP surfaces, not placeholder behavior.

## Promotion Checklist

A roadmap item can move into implementation only when all are true:

- Entry condition is satisfied by committed code.
- Required evidence can be produced locally and in CI.
- Security policy does not forbid the operation.
- The implementation stores evidence spans, source metadata, and confidence where relevant.
- The feature can be driven through a real CLI, API, worker, MCP, or integration surface.
