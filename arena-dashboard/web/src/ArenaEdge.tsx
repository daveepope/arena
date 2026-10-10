import { getStraightPath, useStore, type EdgeProps } from "@xyflow/react";

import { getEdgeParams } from "./edges";

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
  const stroke = edgeData?.faulted ? "#e74c3c" : edgeData?.decorative ? "#4a4a4a" : "#6a6a6a";
  const strokeWidth = edgeData?.decorative ? 1 : 2;
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
