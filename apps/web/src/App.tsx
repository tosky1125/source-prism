import { Database, GitBranch, RefreshCw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  loadOverview,
  loadReferences,
  loadSymbols,
  repoIdFromPath,
  searchContext,
} from "./api";
import { CallGraph } from "./components/CallGraph";
import { Metrics } from "./components/Metrics";
import { ReferenceList } from "./components/ReferenceList";
import { SymbolPicker } from "./components/SymbolPicker";
import type {
  Evidence,
  ReferenceRecord,
  RepoId,
  RepoOverview,
  SymbolRecord,
} from "./types";

const EMPTY_EVIDENCE = {} satisfies Evidence;

export function App() {
  const repoId = useMemo(() => repoIdFromPath(window.location.pathname), []);
  const [overview, setOverview] = useState<RepoOverview | null>(null);
  const [symbols, setSymbols] = useState<readonly SymbolRecord[]>([]);
  const [selectedSymbol, setSelectedSymbol] = useState("");
  const [references, setReferences] = useState<readonly ReferenceRecord[]>([]);
  const [query, setQuery] = useState("search_context");
  const [status, setStatus] = useState("Loading repository");

  useEffect(() => {
    void loadRepo(
      repoId,
      setOverview,
      setSymbols,
      setSelectedSymbol,
      setStatus,
    );
  }, [repoId]);

  useEffect(() => {
    if (!selectedSymbol) return;
    void loadSymbolReferences(repoId, selectedSymbol, setReferences, setStatus);
  }, [repoId, selectedSymbol]);

  const evidence = overview?.latest_run?.evidence ?? EMPTY_EVIDENCE;
  const referenceCount = references.filter(
    (item) => item.relation === "calls",
  ).length;

  async function runSearch(searchQuery: string): Promise<void> {
    if (!searchQuery) return;
    setStatus("Searching context");
    try {
      const result = await searchContext(repoId, searchQuery);
      const hit = result.context_pack.hits.find(
        (item) => item.symbol.kind !== "test_case",
      );
      if (hit) setSelectedSymbol(hit.symbol.fqn);
      setStatus(
        `Search hits ${result.hit_count}, chunks ${result.search_chunk_count}`,
      );
    } catch (error) {
      setStatus(errorMessage(error));
    }
  }

  return (
    <main className="app-shell">
      <header className="hero">
        <div>
          <span className="eyebrow">Source Prism</span>
          <h1>Repo intelligence graph</h1>
          <p>{repoId.value}</p>
        </div>
        <div className="hero-actions">
          <a href="#overview">
            <Database size={18} aria-hidden="true" />
            Overview
          </a>
          <a href="#calls">
            <GitBranch size={18} aria-hidden="true" />
            Calls
          </a>
        </div>
      </header>
      <Metrics evidence={evidence} referenceCount={referenceCount} />
      <section className="workspace" id="overview">
        <SymbolPicker
          symbols={symbols}
          selectedSymbol={selectedSymbol}
          query={query}
          onQueryChange={setQuery}
          onSelect={setSelectedSymbol}
          onSearch={(value) => void runSearch(value)}
        />
        <div className="graph-stack" id="calls">
          {selectedSymbol ? (
            <CallGraph
              selectedSymbol={selectedSymbol}
              references={references}
            />
          ) : (
            <div className="empty-state">No callable symbol selected</div>
          )}
          <ReferenceList references={references} />
        </div>
      </section>
      <footer className="status-line">
        <RefreshCw size={16} aria-hidden="true" />
        {status}
      </footer>
    </main>
  );
}

async function loadRepo(
  repoId: RepoId,
  setOverview: (value: RepoOverview) => void,
  setSymbols: (value: readonly SymbolRecord[]) => void,
  setSelectedSymbol: (value: string) => void,
  setStatus: (value: string) => void,
): Promise<void> {
  try {
    const [overview, symbols] = await Promise.all([
      loadOverview(repoId),
      loadSymbols(repoId),
    ]);
    setOverview(overview);
    setSymbols(symbols);
    setSelectedSymbol(firstCallableSymbol(symbols));
    setStatus("Live");
  } catch (error) {
    setStatus(errorMessage(error));
  }
}

async function loadSymbolReferences(
  repoId: RepoId,
  symbol: string,
  setReferences: (value: readonly ReferenceRecord[]) => void,
  setStatus: (value: string) => void,
): Promise<void> {
  try {
    const report = await loadReferences(repoId, symbol);
    setReferences(report.references);
    const callReferences = report.references.filter(
      (reference) => reference.relation === "calls",
    );
    const incomingCalls = callReferences.filter(
      (reference) => reference.direction === "incoming",
    ).length;
    const outgoingCalls = callReferences.filter(
      (reference) => reference.direction === "outgoing",
    ).length;
    setStatus(`Calls in ${incomingCalls} / out ${outgoingCalls}`);
  } catch (error) {
    setReferences([]);
    setStatus(errorMessage(error));
  }
}

function firstCallableSymbol(symbols: readonly SymbolRecord[]): string {
  const callable = symbols.find((symbol) =>
    ["function", "method"].includes(symbol.kind),
  );
  return (
    callable?.fqn ??
    symbols.find((symbol) => symbol.kind !== "test_case")?.fqn ??
    ""
  );
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "Unknown error";
}
