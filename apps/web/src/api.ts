import ky from "ky";
import type { z } from "zod";
import {
  type ContextSearch,
  contextSearchSchema,
  type ReferencesResponse,
  type RepoId,
  type RepoOverview,
  referencesResponseSchema,
  repoOverviewSchema,
  type SymbolRecord,
  symbolsResponseSchema,
} from "./types";

const client = ky.create({ timeout: 20_000, retry: { limit: 1 } });

export function repoIdFromPath(pathname: string): RepoId {
  const parts = pathname.split("/").filter(Boolean);
  return { value: decodeURIComponent(parts[1] ?? "source-prism") };
}

export async function loadOverview(repoId: RepoId): Promise<RepoOverview> {
  return parsed(
    repoOverviewSchema,
    `/v1/repos/${encodeURIComponent(repoId.value)}`,
  );
}

export async function loadSymbols(
  repoId: RepoId,
): Promise<readonly SymbolRecord[]> {
  const body = await parsed(symbolsResponseSchema, api(repoId, "symbols"));
  return body.symbols;
}

export async function loadReferences(
  repoId: RepoId,
  symbol: string,
): Promise<ReferencesResponse> {
  const url = `${api(repoId, "references")}?symbol=${encodeURIComponent(symbol)}`;
  return parsed(referencesResponseSchema, url);
}

export async function searchContext(
  repoId: RepoId,
  query: string,
): Promise<ContextSearch> {
  const body = await client
    .post(`/v1/repos/${encodeURIComponent(repoId.value)}/context/search`, {
      json: { query },
    })
    .json<unknown>();
  return contextSearchSchema.parse(body);
}

function api(repoId: RepoId, path: string): string {
  return `/v1/repos/${encodeURIComponent(repoId.value)}/${path}`;
}

async function parsed<T extends z.ZodType>(
  schema: T,
  url: string,
): Promise<z.infer<T>> {
  const body = await client.get(url).json<unknown>();
  return schema.parse(body);
}
