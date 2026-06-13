import { ArrowDownLeft, ArrowUpRight } from "lucide-react";
import type { ReferenceRecord } from "../types";

type ReferenceListProps = {
  readonly references: readonly ReferenceRecord[];
};

export function ReferenceList({ references }: ReferenceListProps) {
  const incoming = references.filter((item) => item.direction === "incoming");
  const outgoing = references.filter((item) => item.direction === "outgoing");

  return (
    <section className="reference-panel">
      <ReferenceColumn
        title="Incoming"
        icon="incoming"
        references={incoming}
        emptyText="No direct callers"
      />
      <ReferenceColumn
        title="Outgoing"
        icon="outgoing"
        references={outgoing}
        emptyText="No direct callees"
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
  readonly icon: "incoming" | "outgoing";
  readonly references: readonly ReferenceRecord[];
  readonly emptyText: string;
}) {
  const Icon = icon === "incoming" ? ArrowDownLeft : ArrowUpRight;
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
          <span>
            {reference.confidence} / {reference.file_path}
          </span>
        </div>
      ))}
    </article>
  );
}

function referenceKey(reference: ReferenceRecord): string {
  return `${reference.direction}:${reference.source_fqn}:${reference.target_fqn}:${reference.file_path}`;
}
