# Production Hardening Audit

Generated: 2026-06-13
Scope: R6 production-hardening status.

## Result

Status: R6 complete for the current local-only platform.

The product still has no non-local deployment mode. That is intentional:
public bind addresses are rejected until the auth/tenancy design in
`docs/auth-tenancy.md` is implemented.

## Controls

- Non-local bind guard:
  - `crates/ri-config/tests/config.rs`
  - `crates/ri-api/tests/bind_addr.rs`
  - real CLI/API startup smokes from the R6 bind-guard slice
- Request body limit:
  - `crates/ri-api/tests/request_limits.rs`
- Rate limit:
  - `crates/ri-api/src/rate_limit.rs`
  - `crates/ri-api/tests/request_limits.rs`
  - real HTTP 429 smoke from the R6 rate-limit slice
- Dead-letter/job observability:
  - `GET /v1/repos/{repo_id}/dead-letters`
  - `ri-cli dead-letters --repo-id <repo_id>`
  - `crates/ri-api/tests/repo_dead_letters.rs`
  - `crates/ri-cli/tests/dead_letters.rs`
- Search drift hardening:
  - `crates/ri-indexer/tests/search_drift.rs`
  - `scripts/ci/smoke-api.sh`
- Review/publisher redaction:
  - `crates/ri-review/tests/redaction.rs`
  - `crates/ri-github/tests/dry_run.rs`
  - `crates/ri-gitlab/tests/dry_run.rs`

## Log and Secret Boundary

Current runtime code does not log request bodies, source text, repo evidence,
or configured database/search URLs. Secret-bearing review text is redacted
before GitHub/GitLab dry-run payload serialization. Config error paths expose
keys, not values.

Follow-up before non-local mode:

- add structured request IDs,
- add tenant/principal IDs to audit logs,
- keep request bodies and repo text out of logs,
- run token scans against logs, context packs, OpenSearch docs, and publisher
  artifacts.

## Non-Local Deployment Decision

Decision: blocked.

Reason: Source Prism exposes repository intelligence. Non-local deployment
requires authentication, tenant ownership, authorization checks, audit logs,
and tenant-aware rate limits. The current local-only guard is the correct
production safety boundary until those are implemented.
