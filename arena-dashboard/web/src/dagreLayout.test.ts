import "./testSetup";

import { describe, expect, it } from "vitest";

import { dagreLayout, estimateNodeWidth } from "./dagreLayout";
import type { LayoutEdge, LayoutNode } from "./layout";

function node(id: string): LayoutNode {
  return { id, label: id, kind: "dependency", state: "started", faults: [], childCount: 0, visible: true, componentIds: [], depth: 0 };
}

function edge(source: string, target: string): LayoutEdge {
  return { id: `${source}->${target}`, source, target, faulted: false, decorative: false, visible: true };
}

describe("dagreLayout", () => {
  it("returns a position for every node", () => {
    const nodes = [node("a"), node("b"), node("c")];
    const positions = dagreLayout(nodes, [edge("a", "b"), edge("b", "c")]);

    expect(positions.size).toBe(3);
    for (const n of nodes) {
      const position = positions.get(n.id);
      expect(position).toBeDefined();
      expect(Number.isFinite(position?.x)).toBe(true);
      expect(Number.isFinite(position?.y)).toBe(true);
    }
  });

  it("places an edge's source below its target (bottom-to-top layout)", () => {
    const positions = dagreLayout([node("child"), node("parent")], [edge("child", "parent")]);

    const childY = positions.get("child")?.y ?? 0;
    const parentY = positions.get("parent")?.y ?? 0;
    expect(childY).toBeGreaterThan(parentY);
  });

  it("nodes with no edges still receive positions", () => {
    const positions = dagreLayout([node("isolated")], []);

    expect(positions.get("isolated")).toBeDefined();
  });
});

describe("estimateNodeWidth", () => {
  it("short label clamps to the minimum width", () => {
    expect(estimateNodeWidth("a")).toBe(100);
  });

  it("long label clamps to the maximum width", () => {
    expect(estimateNodeWidth("arena-executable-component-example-api-web-app-khr7yx")).toBe(260);
  });

  it("profilerChipWiderThanTheLabelDrivesTheWidth", () => {
    expect(estimateNodeWidth("svc", "cpu profile report")).toBe(134);
  });

  it("mid-length label scales with its character count", () => {
    expect(estimateNodeWidth("orders-db (postgres)")).toBe(140);
  });
});
