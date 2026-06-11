# Security and Trust Boundary

Source Prism treats repositories, pull requests, comments, docs, and CI artifacts as untrusted input. The foundation milestone indexes repository evidence only; it does not execute target repository code.

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
- Publishing review findings without file, line, evidence, impact path, and actionable recommendation.

## Indexing

Indexing is read-only with respect to the target repository. It may create Source Prism records for manifests, generations, symbols, graph nodes, graph edges, chunks, and evidence. It must not modify target files or execute target code while producing those records.

Generated data uses deterministic IDs and soft stale retirement. This keeps indexing idempotent and avoids deleting historical evidence during normal refreshes.

## LSP and SCIP

SCIP import is allowed when the index file is provided as an artifact. Live LSP queries are a later milestone and must run with the same untrusted-input model: no secrets, bounded time, bounded memory, and no implicit execution of target repository code.

Language servers that require package installation, build scripts, or project execution are not allowed in the foundation milestone. They need an explicit sandbox design first.

## Test and Refactor Execution

Test execution and automated refactor execution are deferred until sandbox design exists. Future execution must use a restricted workspace, no secrets, bounded CPU and memory, controlled network access, and artifact-only outputs.

Refactor planning may recommend characterization tests and PR slicing, but the executor must not create branches, run codemods, or run target tests until the sandbox policy is implemented.

## Prompt Injection

All repository content is untrusted evidence. Context packs must separate trusted Source Prism instructions from untrusted PR descriptions, comments, code, docs, and retrieved chunks.

LLMs may use untrusted content only as repository evidence. They must not follow instructions embedded inside code comments, docs, issue text, or PR descriptions.

## MCP and External Integrations

MCP tools must expose evidence-bound repo queries, not arbitrary shell execution. GitHub and GitLab integrations may publish Source Prism-owned findings only after deterministic verification.

Provider adapters for LLMs and embeddings must receive the minimum required context and must preserve trust labels so downstream review and refactor layers can reject unsupported claims.
