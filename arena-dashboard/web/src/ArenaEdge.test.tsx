import "./testSetup";

import { render } from "@testing-library/react";
import { ReactFlowProvider, useStoreApi, type EdgeProps, type InternalNode } from "@xyflow/react";
import { useEffect, useState, type ReactNode } from "react";
import { describe, expect, it } from "vitest";

import { ArenaEdge, type ArenaEdgeData } from "./ArenaEdge";

const FAULTED_STROKE = "rgb(244, 63, 94)";
const LINK_STROKE = "rgb(100, 116, 139)";
const STRUCTURAL_STROKE = "rgb(148, 163, 184)";

function internalNode(id: string, x: number, y: number): InternalNode {
  return {
    id,
    position: { x, y },
    measured: { width: 80, height: 40 },
    internals: { positionAbsolute: { x, y } },
  } as unknown as InternalNode;
}

function SeedNodeLookup({ nodes, children }: { nodes: InternalNode[]; children: ReactNode }) {
  const store = useStoreApi();
  const [seeded, setSeeded] = useState(false);
  useEffect(() => {
    store.setState({ nodeLookup: new Map(nodes.map((n) => [n.id, n])) } as never);
    setSeeded(true);
  }, [store, nodes]);
  return seeded ? <>{children}</> : null;
}

function edgeProps(data?: Partial<ArenaEdgeData>): EdgeProps {
  return {
    id: "a>b",
    source: "a",
    target: "b",
    data,
    markerEnd: "url(#arrow)",
  } as unknown as EdgeProps;
}

function renderEdge(nodes: InternalNode[], data?: Partial<ArenaEdgeData>): SVGPathElement | null {
  const { container } = render(
    <ReactFlowProvider>
      <SeedNodeLookup nodes={nodes}>
        <svg>
          <ArenaEdge {...edgeProps(data)} />
        </svg>
      </SeedNodeLookup>
    </ReactFlowProvider>
  );
  return container.querySelector("path");
}

function bothNodes(): InternalNode[] {
  return [internalNode("a", 0, 0), internalNode("b", 300, 0)];
}

describe("ArenaEdge", () => {
  it("both endpoints present renders a path carrying the edge id and marker", () => {
    const path = renderEdge(bothNodes(), { faulted: false, decorative: false, visible: true });

    expect(path).not.toBeNull();
    expect(path?.getAttribute("id")).toBe("a>b");
    expect(path?.getAttribute("marker-end")).toBe("url(#arrow)");
    expect(path?.getAttribute("class")).toBe("react-flow__edge-path");
  });

  it("both endpoints present draws the path between their intersection points", () => {
    const path = renderEdge(bothNodes(), { faulted: false, decorative: false, visible: true });

    expect(path?.getAttribute("d")).toBe("M 80,20L 300,20");
  });

  it("missing source node renders nothing", () => {
    const path = renderEdge([internalNode("b", 300, 0)], { faulted: false, decorative: false, visible: true });

    expect(path).toBeNull();
  });

  it("missing target node renders nothing", () => {
    const path = renderEdge([internalNode("a", 0, 0)], { faulted: false, decorative: false, visible: true });

    expect(path).toBeNull();
  });

  it.each([
    ["faulted", { faulted: true, decorative: false }, FAULTED_STROKE, "2", ""],
    ["faulted and decorative", { faulted: true, decorative: true }, FAULTED_STROKE, "1.75", "4 4"],
    ["decorative", { faulted: false, decorative: true }, LINK_STROKE, "1.75", "4 4"],
    ["structural", { faulted: false, decorative: false }, STRUCTURAL_STROKE, "2", ""],
  ])("%s edge strokes in its own colour, width and dash pattern", (_label, data, stroke, width, dash) => {
    const path = renderEdge(bothNodes(), { ...data, visible: true });

    expect(path?.style.stroke).toBe(stroke);
    expect(path?.style.strokeWidth).toBe(width);
    expect(path?.style.strokeDasharray).toBe(dash);
  });

  it.each([
    ["visible", true, "1"],
    ["hidden", false, "0"],
  ])("%s edge renders at the matching opacity", (_label, visible, opacity) => {
    const path = renderEdge(bothNodes(), { faulted: false, decorative: false, visible });

    expect(path?.style.opacity).toBe(opacity);
  });

  it("absent data renders a hidden structural edge", () => {
    const path = renderEdge(bothNodes());

    expect(path?.style.stroke).toBe(STRUCTURAL_STROKE);
    expect(path?.style.strokeWidth).toBe("2");
    expect(path?.style.strokeDasharray).toBe("");
    expect(path?.style.opacity).toBe("0");
  });
});
