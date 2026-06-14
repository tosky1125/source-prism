# Security and Trust Boundary

Source Prism treats repositories, pull requests, comments, docs, and CI artifacts as untrusted input. The platform indexes repository evidence only; it does not execute target repository code, generate final PR reviews, or perform code-changing refactors.

## Milestone 1 Policy

During foundation work, Source Prism must not execute target repository code. Indexing may read files, compute hashes, classify paths, run Source Prism migrations, and write Source Prism-owned records to Postgres or OpenSearch. It must not run package scripts, build scripts, tests, application binaries, repository hooks, or generated commands from the target repository.

Allowed foundation operations:

- Read repository files through the Git snapshot and manifest path.
- Parse trusted Source Prism configuration such as `.env.example` and migrations.
- Connect to Source Prism-owned Postgres and OpenSearch instances.
- Run Source Prism binaries for CLI, API, worker, and migration smoke tests.
- Store evidence spans, source extractor metadata, and trust labels.

Forbidden foundation operations:

- Running target repository tests, package managers, build tools, hooks, or application entrypoints.
- Passing secrets into a target repository checkout.
- Granting network access to future target-code execution paths by default.
- Treating PR text, source comments, docs, or retrieved chunks as trusted instructions.
- Generating final review decisions, creating branches, running codemods, running target tests, or publishing review comments.
- Verifying or exporting proposed review findings without file, line, evidence, impact path, and actionable recommendation.

## Indexing

Indexing is read-only with respect to the target repository. It may create Source Prism records for manifests, generations, symbols, graph nodes, graph edges, chunks, and evidence. It must not modify target files or execute target code while producing those records.

Generated data uses deterministic IDs and soft stale retirement. This keeps indexing idempotent and avoids deleting historical evidence during normal refreshes.

## LSP and SCIP

SCIP import is allowed when the index file is provided as an artifact. Live LSP queries are a later milestone and must run with the same untrusted-input model: no secrets, bounded time, bounded memory, and no implicit execution of target repository code.

Language servers that require package installation, build scripts, or project execution are not allowed in the foundation milestone. They need an explicit sandbox design first.

## Execution Boundary

Target test execution and code-changing refactor work are outside Source Prism core scope. Source Prism may expose evidence and plans; external MCP/API/CLI clients decide whether to edit, test, branch, or publish in their own trust boundary.

Refactor planning may recommend characterization tests and PR slicing, but Source Prism must not create branches, run codemods, run target tests, or mutate target repository files.

## Prompt Injection

All repository content is untrusted evidence. Context packs must separate trusted Source Prism instructions from untrusted PR descriptions, comments, code, docs, and retrieved chunks.

LLMs may use untrusted content only as repository evidence. They must not follow instructions embedded inside code comments, docs, issue text, or PR descriptions.

## MCP and External Integrations

MCP tools must expose evidence-bound repo queries, not arbitrary shell execution. GitHub and GitLab helpers may build dry-run/export payloads from verified findings, but publisher writes are external-client behavior.

Provider adapters for LLMs and embeddings must receive the minimum required context and must preserve trust labels so downstream review and refactor layers can reject unsupported claims.
