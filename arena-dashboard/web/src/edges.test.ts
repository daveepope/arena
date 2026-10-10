import type { InternalNode } from "@xyflow/react";
import { Position } from "@xyflow/react";
import { describe, expect, it } from "vitest";

import { getEdgeParams } from "./edges";

function fakeNode(x: number, y: number, size = 80): InternalNode {
  return {
    id: `node-${x}-${y}`,
    position: { x, y },
    measured: { width: size, height: size },
    internals: { positionAbsolute: { x, y } },
  } as unknown as InternalNode;
}

describe("getEdgeParams", () => {
  it("horizontally adjacent nodes connect right-to-left", () => {
    const left = fakeNode(0, 0);
    const right = fakeNode(200, 0);

    const params = getEdgeParams(left, right);

    expect(params.sourcePos).toBe(Position.Right);
    expect(params.targetPos).toBe(Position.Left);
    expect(Number.isFinite(params.sx)).toBe(true);
    expect(Number.isFinite(params.sy)).toBe(true);
    expect(Number.isFinite(params.tx)).toBe(true);
    expect(Number.isFinite(params.ty)).toBe(true);
  });

  it("vertically stacked nodes connect bottom-to-top", () => {
    const bottom = fakeNode(0, 200);
    const top = fakeNode(0, 0);

    const params = getEdgeParams(bottom, top);

    expect(params.sourcePos).toBe(Position.Top);
    expect(params.targetPos).toBe(Position.Bottom);
  });

  it("diagonally offset nodes still return finite coordinates", () => {
    const params = getEdgeParams(fakeNode(0, 0), fakeNode(150, 150));

    expect(Number.isFinite(params.sx)).toBe(true);
    expect(Number.isFinite(params.tx)).toBe(true);
  });
});
