import type { Evidence } from "../types";

type MetricsProps = {
  readonly evidence: Evidence;
  readonly referenceCount: number;
};

const METRICS = [
  ["file_manifests", "Files"],
  ["symbols", "Symbols"],
  ["graph_edges", "Edges"],
  ["search_chunks", "Search chunks"],
  ["test_cases", "Tests"],
  ["architecture_entities", "Docs"],
] as const;

export function Metrics({ evidence, referenceCount }: MetricsProps) {
  return (
    <section className="metrics" aria-label="Repository metrics">
      {METRICS.map(([key, label]) => (
        <article className="metric" key={key}>
          <strong>{evidence[key] ?? 0}</strong>
          <span>{label}</span>
        </article>
      ))}
      <article className="metric metric-accent">
        <strong>{referenceCount}</strong>
        <span>Visible calls</span>
      </article>
    </section>
  );
}
