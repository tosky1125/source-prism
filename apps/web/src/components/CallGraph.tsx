import { Background, Controls, MiniMap, ReactFlow } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Network } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { buildCallGraph } from "../graph";
import type { ReferenceRecord } from "../types";

type CallGraphProps = {
  readonly selectedSymbol: string;
  readonly references: readonly ReferenceRecord[];
};

export function CallGraph({ selectedSymbol, references }: CallGraphProps) {
  const layout = useGraphLayout();
  const graph = useMemo(
    () => buildCallGraph(selectedSymbol, references, layout),
    [selectedSymbol, references, layout],
  );
  const nodeCount = graph.nodes.length;
  const edgeCount = graph.edges.length;

  return (
    <section className="graph-panel" aria-label="Function call graph">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Call graph</span>
          <h2>{selectedSymbol}</h2>
        </div>
        <div className="graph-stats">
          <Network size={18} aria-hidden="true" />
          <span>
            {nodeCount} nodes / {edgeCount} calls
          </span>
        </div>
      </div>
      <div className="flow-shell">
        <ReactFlow
          nodes={graph.nodes}
          edges={graph.edges}
          defaultViewport={{ x: 0, y: 0, zoom: 1 }}
          minZoom={0.25}
          maxZoom={1.6}
          nodesDraggable
          nodesConnectable={false}
          elementsSelectable
        >
          <Background color="#d3cab8" gap={24} />
          <Controls showInteractive={false} />
          <MiniMap pannable zoomable nodeStrokeWidth={3} />
        </ReactFlow>
      </div>
    </section>
  );
}

function useGraphLayout(): "wide" | "narrow" {
  const [layout, setLayout] = useState<"wide" | "narrow">("wide");
  useEffect(() => {
    const query = window.matchMedia("(max-width: 700px)");
    const updateLayout = () => setLayout(query.matches ? "narrow" : "wide");
    updateLayout();
    query.addEventListener("change", updateLayout);
    return () => query.removeEventListener("change", updateLayout);
  }, []);
  return layout;
}
