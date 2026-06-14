import { ArrowDownLeft, ArrowUpRight, ShieldCheck } from "lucide-react";
import type { ReferenceRecord } from "../types";

type ReferenceListProps = {
  readonly references: readonly ReferenceRecord[];
};

export function ReferenceList({ references }: ReferenceListProps) {
  const calls = references.filter((item) => item.relation === "calls");
  const incomingCalls = calls.filter((item) => item.direction === "incoming");
  const outgoingCalls = calls.filter((item) => item.direction === "outgoing");
  const testCoverage = references.filter(
    (item) => item.relation === "test_covers",
  );

  return (
    <section className="reference-panel">
      <ReferenceColumn
        title="Incoming calls"
        icon="incoming"
        references={incomingCalls}
        emptyText="No direct callers"
      />
      <ReferenceColumn
        title="Outgoing calls"
        icon="outgoing"
        references={outgoingCalls}
        emptyText="No direct callees"
      />
      <ReferenceColumn
        title="Related tests"
        icon="tests"
        references={testCoverage}
        emptyText="No related tests"
      />
    </section>
  );
}

function ReferenceColumn({
  title,
  icon,
  references,
  emptyText,
}: {
  readonly title: string;
  readonly icon: "incoming" | "outgoing" | "tests";
  readonly references: readonly ReferenceRecord[];
  readonly emptyText: string;
}) {
  const Icon = iconFor(icon);
  return (
    <article className="reference-column">
      <h3>
        <Icon size={16} aria-hidden="true" />
        {title}
      </h3>
      {references.length === 0 ? <p className="empty">{emptyText}</p> : null}
      {references.slice(0, 12).map((reference) => (
        <div className="reference-row" key={referenceKey(reference)}>
          <strong>
            {reference.source_fqn} → {reference.target_fqn}
          </strong>
          <span className="reference-relation">
            {relationLabel(reference.relation)}
          </span>
          <span>
            {reference.confidence} / {reference.file_path}
          </span>
        </div>
      ))}
    </article>
  );
}

function iconFor(icon: "incoming" | "outgoing" | "tests") {
  if (icon === "incoming") return ArrowDownLeft;
  if (icon === "outgoing") return ArrowUpRight;
  return ShieldCheck;
}

function relationLabel(relation: string): string {
  if (relation === "calls") return "call";
  if (relation === "test_covers") return "test coverage";
  return relation;
}

function referenceKey(reference: ReferenceRecord): string {
  return `${reference.relation}:${reference.direction}:${reference.source_fqn}:${reference.target_fqn}:${reference.file_path}`;
}
