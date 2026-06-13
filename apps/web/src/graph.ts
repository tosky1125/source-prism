import type { Edge, Node } from "@xyflow/react";
import type { ReferenceRecord } from "./types";

export type CallNodeData = {
  readonly label: string;
  readonly role: "caller" | "selected" | "callee";
  readonly filePath?: string;
};

export type CallNode = Node<CallNodeData>;
export type CallEdge = Edge<{ readonly relation: string }>;
export type GraphLayout = "wide" | "narrow";

export function buildCallGraph(
  selectedSymbol: string,
  references: readonly ReferenceRecord[],
  layout: GraphLayout,
): { readonly nodes: CallNode[]; readonly edges: CallEdge[] } {
  const callReferences = references.filter((item) => item.relation === "calls");
  const incoming = callReferences.filter(
    (item) => item.direction === "incoming",
  );
  const outgoing = callReferences.filter(
    (item) => item.direction === "outgoing",
  );
  const nodeMap = new Map<string, CallNode>();
  const visibleIncoming = incoming.slice(0, 12);
  const visibleOutgoing = outgoing.slice(0, 12);
  addNode(
    nodeMap,
    selectedSymbol,
    "selected",
    layoutX(layout, "selected"),
    210,
  );

  visibleIncoming.forEach((item, index) => {
    addNode(
      nodeMap,
      item.source_fqn,
      "caller",
      layoutX(layout, "caller"),
      stackY(layout, "caller", index, visibleIncoming.length),
      item.file_path,
    );
  });
  visibleOutgoing.forEach((item, index) => {
    addNode(
      nodeMap,
      item.target_fqn,
      "callee",
      layoutX(layout, "callee"),
      stackY(layout, "callee", index, visibleOutgoing.length),
      item.file_path,
    );
  });

  const edges = callReferences.slice(0, 24).map((item) => edgeFor(item));
  return { nodes: [...nodeMap.values()], edges };
}

function layoutX(layout: GraphLayout, role: CallNodeData["role"]): number {
  if (layout === "narrow") return 70;
  if (role === "caller") return 70;
  if (role === "callee") return 650;
  return 360;
}

function stackY(
  layout: GraphLayout,
  role: CallNodeData["role"],
  index: number,
  total: number,
): number {
  if (layout === "narrow" && role === "caller") return 35 + index * 92;
  if (layout === "narrow" && role === "callee") return 385 + index * 92;
  return 210 + (index - (total - 1) / 2) * 92;
}

function addNode(
  nodes: Map<string, CallNode>,
  label: string,
  role: CallNodeData["role"],
  x: number,
  y: number,
  filePath?: string,
): void {
  if (nodes.has(label)) return;
  nodes.set(label, {
    id: label,
    position: { x, y },
    data: nodeData(label, role, filePath),
    className: `call-node call-node-${role}`,
  });
}

function nodeData(
  label: string,
  role: CallNodeData["role"],
  filePath: string | undefined,
): CallNodeData {
  if (filePath) return { label, role, filePath };
  return { label, role };
}

function edgeFor(reference: ReferenceRecord): CallEdge {
  return {
    id: `${reference.source_fqn}->${reference.target_fqn}`,
    source: reference.source_fqn,
    target: reference.target_fqn,
    animated: reference.confidence !== "low",
    label: reference.relation,
    data: { relation: reference.relation },
  };
}
