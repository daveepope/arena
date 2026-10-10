import { Position, type InternalNode, type Node, type XYPosition } from "@xyflow/react";

function getNodeIntersection(intersectionNode: InternalNode, targetNode: InternalNode): XYPosition {
  const { internals: intersectionInternals } = intersectionNode;
  const { width: intersectionNodeWidth, height: intersectionNodeHeight } = intersectionNode.measured ?? {
    width: 0,
    height: 0,
  };
  const targetPosition = targetNode.internals.positionAbsolute;

  const w = (intersectionNodeWidth ?? 0) / 2;
  const h = (intersectionNodeHeight ?? 0) / 2;

  const x2 = intersectionInternals.positionAbsolute.x + w;
  const y2 = intersectionInternals.positionAbsolute.y + h;
  const x1 = targetPosition.x + w;
  const y1 = targetPosition.y + h;

  const xx1 = (x1 - x2) / (2 * w) - (y1 - y2) / (2 * h);
  const yy1 = (x1 - x2) / (2 * w) + (y1 - y2) / (2 * h);
  const a = 1 / (Math.abs(xx1) + Math.abs(yy1));
  const xx3 = a * xx1;
  const yy3 = a * yy1;
  const x = w * (xx3 + yy3) + x2;
  const y = h * (-xx3 + yy3) + y2;

  return { x, y };
}

function getEdgePosition(node: Node, intersectionPoint: XYPosition): Position {
  const n = { ...node.position, ...node };
  const nx = Math.round(n.x);
  const ny = Math.round(n.y);
  const px = Math.round(intersectionPoint.x);
  const py = Math.round(intersectionPoint.y);

  if (px <= nx + 1) return Position.Left;
  if (px >= nx + (n.measured?.width ?? 0) - 1) return Position.Right;
  if (py <= ny + 1) return Position.Top;
  if (py >= n.y + (n.measured?.height ?? 0) - 1) return Position.Bottom;
  return Position.Top;
}

export function getEdgeParams(source: InternalNode, target: InternalNode) {
  const sourceIntersectionPoint = getNodeIntersection(source, target);
  const targetIntersectionPoint = getNodeIntersection(target, source);

  const sourcePos = getEdgePosition(source, sourceIntersectionPoint);
  const targetPos = getEdgePosition(target, targetIntersectionPoint);

  return {
    sx: sourceIntersectionPoint.x,
    sy: sourceIntersectionPoint.y,
    tx: targetIntersectionPoint.x,
    ty: targetIntersectionPoint.y,
    sourcePos,
    targetPos,
  };
}
