# Roadmap Guardrails

Foundation is the platform skeleton: workspace, configuration, migrations, Git manifests, generation bookkeeping, durable jobs, local API health, search sync contracts, CLI command surface, CI, and security boundaries. Everything below is Roadmap only until its entry condition and evidence gate are satisfied.

## Rule

Roadmap only means the crate or feature may have a placeholder command, schema anchor, trait, or document, but it is not a milestone-1 implementation task. It must not be treated as production behavior until the required evidence exists.

## Post-Foundation Waves

| Area | Crates | Entry condition | Required evidence | Why not created in foundation |
| --- | --- | --- | --- | --- |
| Symbol extraction | `ri-parser`, `ri-tree-sitter`, `ri-symbols` | Stable file manifest and generation lifecycle are available | Tree-sitter fixture suite, changed-line to innermost-symbol mapping, language coverage report | Foundation only proves repo read and manifest paths; parser correctness needs language fixtures |
| Dependency graph and impact | `ri-graph`, `ri-impact` | Symbol records and rough references exist | Edge fixtures with confidence and evidence spans, traversal tests, impact CLI smoke | Graph behavior without symbols would be fake structure |
| Hybrid search and chunks | `ri-search`, `ri-embedding` | Symbol/test/doc chunk plans exist and OpenSearch sync is stable | Exact identifier, BM25, vector, and drift-check fixtures with stale retirement proof | Foundation only proves sync contracts, not retrieval quality |
| SCIP and LSP references | `ri-scip`, `ri-lsp` | Rough symbol/reference layer exists | SCIP import fixtures, LSP timeout/failure tests, confidence calibration | Precise references need sandbox and language-specific behavior |
| Architecture evidence | `ri-architecture` | Parser and manifest paths can locate contracts | OpenAPI, GraphQL, DB migration, event, CODEOWNERS, ADR fixtures | Foundation cannot infer architecture without extractors |
| Behavior and tests | `ri-behavior` | Test artifacts are available without executing target code | JUnit, LCOV, Cobertura, JaCoCo, PHPUnit, pytest, Playwright ingestion fixtures | Foundation forbids target repository execution |
| MCP integration | `ri-mcp` | Query APIs return evidence-bound symbols, graph, search, and test context | MCP tool smoke, trust-boundary tests, no arbitrary shell execution proof | Agent tools need real query surfaces to expose |
| GitHub and GitLab integration | `ri-github`, `ri-gitlab` | Review findings have verifier-backed evidence | Webhook fixture, check/comment dry run, SARIF output proof | Publishing without verified findings would create noise |
| Review and refactor reasoning | `ri-context`, `ri-review`, `ri-refactor` | Impact, retrieval, architecture, and behavior evidence exist | Context pack snapshots, verifier rejection tests, replay evaluation | LLM reasoning is downstream of repo structure, not the core foundation |
| Refactor execution | `ri-refactor` executor path | Sandbox design is implemented and approved | Branch creation dry run, codemod fixture, test/typecheck sandbox proof, rollback plan | Foundation explicitly forbids target code execution |
| Evaluation | `ri-eval` | Review/refactor outputs are available | Golden dataset schema, offline replay, useful/false-positive metrics | Evaluation needs real findings to score |

## Future Crate Creation

Create a new crate only when its boundary has real code, tests, and observable behavior. A planned crate name alone is not enough.

Roadmap-only crates may appear in documentation and command placeholders. They should not be added to the workspace until at least one milestone story needs compiled Rust code in that boundary.

## Command Guardrails

Foundation CLI commands may expose placeholders for future behavior. Those commands must return machine-readable `not_implemented` status and a non-zero exit code until the roadmap entry condition is met.

Current placeholder surfaces that still run in-memory over the local worktree:

- `ri-cli symbols`
- `ri-cli impact`

## Promotion Checklist

A roadmap item can move into implementation only when all are true:

- Entry condition is satisfied by committed code.
- Required evidence can be produced locally and in CI.
- Security policy does not forbid the operation.
- The implementation stores evidence spans, source metadata, and confidence where relevant.
- The feature can be driven through a real CLI, API, worker, MCP, or integration surface.
