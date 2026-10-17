import dagre from "dagre";

import type { LayoutEdge, LayoutNode } from "./layout";

const MIN_NODE_WIDTH = 100;
const MAX_NODE_WIDTH = 260;
const NODE_HEIGHT = 70;
const CHAR_WIDTH_PX = 7;
const CHIP_CHAR_WIDTH_PX = 6;
const CHIP_PADDING_PX = 26;

export function estimateNodeWidth(label: string, chipLabel?: string): number {
  const labelWidth = label.length * CHAR_WIDTH_PX;
  const chipWidth = chipLabel ? chipLabel.length * CHIP_CHAR_WIDTH_PX + CHIP_PADDING_PX : 0;
  return Math.min(MAX_NODE_WIDTH, Math.max(MIN_NODE_WIDTH, labelWidth, chipWidth));
}

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
    g.setNode(n.id, { width: estimateNodeWidth(n.label), height: NODE_HEIGHT });
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
