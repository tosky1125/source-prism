# RI-CLI KNOWLEDGE

## OVERVIEW
`ri-cli` is the local operator surface for indexing, querying evidence, MCP calls, tests, review dry-runs, and refactor planning.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Command dispatch | `src/main.rs` | Subcommand routing |
| Index command | `src/index.rs`, `src/index_args.rs` | Repo path/SHA handling |
| JSON output | `src/index_output.rs` and command modules | Preserve machine-readable stdout |
| Search sync/drift | `src/search.rs`, `src/repo_search_drift.rs` | OpenSearch operations |
| MCP local calls | `src/mcp.rs`, `src/mcp_handler.rs` | Tool catalog and single-call mode |
| Review/refactor | `src/review.rs`, `src/refactor.rs` | Dry-run payloads and planner-only execution |
| CLI tests | `tests/` | Snapshot-like behavior via parsed JSON |

## CONVENTIONS

- Stdout is structured JSON for successful data commands.
- Human errors go to stderr through app-boundary errors.
- Support both worktree mode (`--repo .`) and persisted mode (`--repo-id ...`) where the command advertises it.
- Keep smoke symbols realistic; CI uses `search_context_command`.
- `refactor` remains planner-only and must report execution disabled.
- Test/import commands ingest evidence; they must not execute target repository code.

## COMMANDS

```bash
cargo test -p ri-cli
cargo run -p ri-cli -- repo manifest --repo .
cargo run -p ri-cli -- symbols --repo .
cargo run -p ri-cli -- impact --repo . --symbol search_context_command
cargo run -p ri-cli -- mcp tools
```

## ANTI-PATTERNS

- Do not print logs or progress lines to stdout before JSON payloads.
- Do not make examples depend on fake symbols such as `InvoiceService::applyTax`.
- Do not treat `outgoing = 0` as unused-code proof; check incoming calls and public entrypoints.
