# WEB APP KNOWLEDGE

## OVERVIEW
`apps/web` is the React repository explorer served by `ri-api`.

## STRUCTURE

```text
src/App.tsx             # view state, data loading, route-level composition
src/api.ts              # typed API client helpers
src/types.ts            # API response shapes used by UI
src/graph.ts            # call graph layout helpers
src/components/         # explorer panels and graph widgets
scripts/copy-build.ts   # copies Vite output into ri-api assets
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Repository overview | `src/App.tsx` | Current first screen |
| Call visualization | `src/components/CallGraph.tsx` | Uses `@xyflow/react` |
| Reference grouping | `src/components/ReferenceList.tsx` | Separates calls from test evidence |
| Symbol selection | `src/components/SymbolPicker.tsx` | Search/select behavior |
| API schemas | `src/types.ts` | Keep in sync with `ri-api` JSON |

## CONVENTIONS

- Use TypeScript modules; avoid returning to large inline HTML strings.
- Use `ky` for HTTP and `zod` only when runtime validation is needed.
- Use `lucide-react` for tool icons when available.
- Keep UI dense and operational; this is a repo explorer, not a landing page.
- Build output must be committed under `crates/ri-api/assets/repo-explorer` after UI changes.
- Do not edit generated assets by hand; change `src`, then run the build.

## COMMANDS

```bash
bun run check
bun run build
```

## ANTI-PATTERNS

- Do not show `test_covers` as if it were a direct function call.
- Do not put core UI state in generated `crates/ri-api/assets` files.
- Do not rely on `node_modules` content in repo indexing or documentation.
