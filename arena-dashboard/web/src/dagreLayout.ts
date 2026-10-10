import dagre from "dagre";

import type { LayoutEdge, LayoutNode } from "./layout";

const NODE_SIZE = 90;

// Ported from xyflow's own official "Layouting" example
// (examples/react/src/examples/Layouting/index.tsx in xyflow/xyflow) - the
// documented pattern for auto-laying-out a React Flow graph with dagre.
// rankdir "BT" (bottom-to-top): edge sources (dependencies/nested children)
// rank below edge targets (the components/parents that wait on them), so
// the tree builds up from the bottom with components at the top.
export function dagreLayout(nodes: LayoutNode[], edges: LayoutEdge[]): Map<string, { x: number; y: number }> {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: "BT", nodesep: 60, ranksep: 90 });

  nodes.forEach((n) => {
    g.setNode(n.id, { width: NODE_SIZE, height: NODE_SIZE });
  });
  edges.forEach((e) => {
    g.setEdge(e.source, e.target);
  });

  dagre.layout(g);

  const positions = new Map<string, { x: number; y: number }>();
  nodes.forEach((n) => {
    const { x, y } = g.node(n.id);
    positions.set(n.id, { x, y });
  });
  return positions;
}
