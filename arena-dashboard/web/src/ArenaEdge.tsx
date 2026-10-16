import { getStraightPath, useStore, type EdgeProps } from "@xyflow/react";

import { getEdgeParams } from "./edges";

const EDGE_FAULTED_COLOR = "#f43f5e";
const EDGE_LINK_COLOR = "#64748b";
const EDGE_STRUCTURAL_COLOR = "#94a3b8";

export interface ArenaEdgeData {
  faulted: boolean;
  decorative: boolean;
  visible: boolean;
}

export function ArenaEdge({ id, source, target, data, markerEnd }: EdgeProps) {
  const { sourceNode, targetNode } = useStore((s) => ({
    sourceNode: s.nodeLookup.get(source),
    targetNode: s.nodeLookup.get(target),
  }));

  if (!sourceNode || !targetNode) return null;

  const { sx, sy, tx, ty } = getEdgeParams(sourceNode, targetNode);
  const [path] = getStraightPath({ sourceX: sx, sourceY: sy, targetX: tx, targetY: ty });

  const edgeData = data as ArenaEdgeData | undefined;
  const stroke = edgeData?.faulted
    ? EDGE_FAULTED_COLOR
    : edgeData?.decorative
      ? EDGE_LINK_COLOR
      : EDGE_STRUCTURAL_COLOR;
  const strokeWidth = edgeData?.decorative ? 1.75 : 2;
  const strokeDasharray = edgeData?.decorative ? "4 4" : undefined;
  const opacity = edgeData?.visible ? 1 : 0;

  return (
    <path
      id={id}
      className="react-flow__edge-path"
      d={path}
      markerEnd={markerEnd}
      style={{ stroke, strokeWidth, strokeDasharray, opacity, transition: "opacity 0.3s" }}
    />
  );
}
