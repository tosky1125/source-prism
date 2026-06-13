import { z } from "zod";

export type RepoId = {
  readonly value: string;
};

export const symbolSchema = z.object({
  stable_symbol_id: z.string(),
  versioned_symbol_id: z.string(),
  file_path: z.string(),
  language: z.string(),
  kind: z.string(),
  name: z.string(),
  fqn: z.string(),
});

export const evidenceSchema = z.object({
  file_manifests: z.number().optional(),
  symbols: z.number().optional(),
  graph_nodes: z.number().optional(),
  graph_edges: z.number().optional(),
  search_chunks: z.number().optional(),
  search_sync_jobs: z.number().optional(),
  test_cases: z.number().optional(),
  architecture_entities: z.number().optional(),
});

export const repoOverviewSchema = z.object({
  status: z.string(),
  kind: z.string(),
  repo: z.object({
    repo_id: z.string(),
    name: z.string(),
    default_branch: z.string(),
  }),
  latest_run: z
    .object({
      run_id: z.string(),
      commit_sha: z.string(),
      index_kind: z.string(),
      status: z.string(),
      started_at: z.string(),
      finished_at: z.string().nullable().optional(),
      evidence: evidenceSchema,
    })
    .nullable()
    .optional(),
});

export const symbolsResponseSchema = z.object({
  kind: z.string(),
  symbol_count: z.number(),
  symbols: z.array(symbolSchema),
});

export const referenceSchema = z.object({
  direction: z.string(),
  relation: z.string(),
  source_fqn: z.string(),
  target_fqn: z.string(),
  file_path: z.string(),
  confidence: z.string(),
  evidence: z.string(),
});

export const referencesResponseSchema = z.object({
  kind: z.string(),
  incoming_count: z.number(),
  outgoing_count: z.number(),
  references: z.array(referenceSchema),
});

export const contextSearchSchema = z.object({
  kind: z.string(),
  hit_count: z.number(),
  impact_count: z.number(),
  search_chunk_count: z.number(),
  bm25_hit_count: z.number(),
  context_pack: z.object({
    hits: z.array(z.object({ symbol: symbolSchema, score: z.number() })),
  }),
});

export type SymbolRecord = z.infer<typeof symbolSchema>;
export type Evidence = z.infer<typeof evidenceSchema>;
export type RepoOverview = z.infer<typeof repoOverviewSchema>;
export type ReferenceRecord = z.infer<typeof referenceSchema>;
export type ReferencesResponse = z.infer<typeof referencesResponseSchema>;
export type ContextSearch = z.infer<typeof contextSearchSchema>;
