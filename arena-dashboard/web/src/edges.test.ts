import "./testSetup";

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

  it("coincident nodes fall back to a top position on both ends", () => {
    const params = getEdgeParams(fakeNode(50, 50), fakeNode(50, 50));

    expect(params.sourcePos).toBe(Position.Top);
    expect(params.targetPos).toBe(Position.Top);
  });

  it.each([
    ["left of", -200, 0, Position.Left, Position.Right],
    ["above", 0, -200, Position.Top, Position.Bottom],
  ])("target %s the source anchors the ends on the facing sides", (_label, tx, ty, sourcePos, targetPos) => {
    const params = getEdgeParams(fakeNode(0, 0), fakeNode(tx as number, ty as number));

    expect(params.sourcePos).toBe(sourcePos);
    expect(params.targetPos).toBe(targetPos);
  });

  it("zero sized nodes yield no usable geometry and fall back to a top position", () => {
    const params = getEdgeParams(fakeNode(0, 0, 0), fakeNode(200, 0, 0));

    expect(Number.isFinite(params.sx)).toBe(false);
    expect(Number.isFinite(params.tx)).toBe(false);
    expect(params.sourcePos).toBe(Position.Top);
    expect(params.targetPos).toBe(Position.Top);
  });
});
