# Source Prism Auth and Tenancy Design

Status: gate design for future non-local deployment.

Source Prism remains local-only until this design is implemented. Current
runtime enforcement rejects non-loopback `API_BIND_ADDR` values in both
`ri-cli config check` and `ri-api` startup.

## Deployment Gate

Non-local deployment is blocked until all gates below are implemented and
verified:

- Authentication: every API request carries a signed service token or user
  session, parsed at the HTTP boundary into a typed principal.
- Tenancy: every repo, run, generation, search chunk, job, and outbox row is
  scoped to one tenant or organization. Cross-tenant IDs are rejected before
  storage or retrieval.
- Authorization: every repo operation checks principal, tenant, repo scope,
  and action. Read, index, review verification, dry-run export, and admin
  actions are distinct capabilities.
- Secret handling: provider tokens are never stored in repo evidence rows,
  never returned through context APIs, and never logged. Source Prism does not
  need publisher-write secrets for its core evidence platform behavior.
- Audit trail: non-local writes record principal, tenant, repo, action,
  target ID, request ID, and result without recording request bodies or source
  text.
- Rate and size controls: request body limits and process rate limits stay on;
  tenant-aware limits replace process-only limits before multi-tenant launch.
- Source execution: target repository code execution, branch creation,
  codemods, target tests, and publisher writes remain outside Source Prism
  core scope.

## Current Local-Only Guarantees

- Default bind address is `127.0.0.1:3000`.
- Non-loopback bind addresses return a config/startup error mentioning
  `auth/tenancy`.
- API request bodies are capped at 256 KiB.
- API request rate limiting defaults to 600 requests per 60 seconds.
- Review dry-run/export payloads redact secret-like text before serialization.
- No API route requires or accepts provider secrets for indexing/search.
- Refactor remains planner-only; Source Prism has no code-changing executor.

## Required Schema Before Activation

Before non-local mode can exist, the canonical schema needs:

- `tenants`
- `principals`
- `principal_tenant_memberships`
- repo ownership columns or join tables
- per-tenant job/search quotas
- audit log rows for mutating API actions

All new rows that can expose repo evidence must include tenant ownership or be
reachable only through a tenant-owned parent.

## Required Public Interfaces

Future API surface:

```text
POST /v1/auth/token/introspect
GET  /v1/me
GET  /v1/tenants/{tenant_id}/repos
```

Future MCP context must carry tenant and principal identity. MCP tools must not
fall back to ambient filesystem or environment authority in non-local mode.

## Verification Required Before Non-Local Mode

- Config tests reject public bind unless auth/tenancy mode is explicitly
  enabled and fully configured.
- API tests reject unauthenticated requests with HTTP 401.
- API tests reject cross-tenant repo access with HTTP 403 or 404.
- SQLx prepare covers tenant-scoped queries.
- Real HTTP smoke proves an authenticated tenant can index/search only its own
  repo.
- Secret scans prove provider tokens do not appear in logs, context packs,
  dry-run/export payloads, or OpenSearch documents.
