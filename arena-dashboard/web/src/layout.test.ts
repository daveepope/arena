import "./testSetup";

import { describe, expect, it } from "vitest";

import { computeLayout } from "./layout";
import type { ArenaState, Subject } from "./types";

function subject(id: string, state: string, children: Subject[] = [], component_ids?: string[]): Subject {
  return { id, state, faults: [], children, component_ids };
}

function arenaState(dependencies: Subject[], components: Subject[]): ArenaState {
  return { id: "arena-1", state: "arena_open", at: "2026-01-01T00:00:00.000Z", dependencies, components, faults: [] };
}

function edgeBetween(layout: ReturnType<typeof computeLayout>, sourceLabel: string, targetLabel: string) {
  const source = layout.nodes.find((n) => n.label === sourceLabel);
  const target = layout.nodes.find((n) => n.label === targetLabel);
  return layout.edges.find((e) => e.source === source?.id && e.target === target?.id);
}

describe("computeLayout", () => {
  it("empty arena produces no nodes", () => {
    const layout = computeLayout(arenaState([], []));

    expect(layout.nodes).toHaveLength(0);
    expect(layout.edges).toHaveLength(0);
  });

  it("top level subjects with no other relationships produce no edges", () => {
    const layout = computeLayout(arenaState([subject("postgres", "started")], [subject("api", "started")]));

    expect(layout.nodes).toHaveLength(2);
    const dep = layout.nodes.find((n) => n.label === "postgres");
    expect(dep?.kind).toBe("dependency");
    expect(dep?.depth).toBe(0);
    const comp = layout.nodes.find((n) => n.label === "api");
    expect(comp?.kind).toBe("component");

    expect(layout.edges).toHaveLength(0);
  });

  it("nested child produces a structural edge from child to parent only", () => {
    const child = subject("http-dep-2", "started");
    const parent = subject("postgres", "started", [child]);
    const layout = computeLayout(arenaState([parent], []));

    expect(layout.nodes).toHaveLength(2);
    expect(layout.edges).toHaveLength(1);
    expect(edgeBetween(layout, "http-dep-2", "postgres")).toBeDefined();
  });

  it("faulted child marks the structural edge as faulted", () => {
    const child = subject("http-dep-2", "faulted");
    const parent = subject("postgres", "started", [child]);
    const layout = computeLayout(arenaState([parent], []));

    expect(edgeBetween(layout, "http-dep-2", "postgres")?.faulted).toBe(true);
  });

  it("only nested subjects get a structural edge to their parent", () => {
    const layout = computeLayout(
      arenaState(
        [
          subject("mssql", "started"),
          subject("http-dep-1", "started"),
          subject("kafka", "started"),
          subject("postgres", "started", [subject("http-dep-2", "started")]),
        ],
        [subject("parent-component", "started", [subject("child-component", "started")])]
      )
    );

    expect(layout.nodes).toHaveLength(7); // 4 top-level deps + 1 nested dep + 2 comps
    expect(edgeBetween(layout, "http-dep-2", "postgres")).toBeDefined();
    expect(edgeBetween(layout, "child-component", "parent-component")).toBeDefined();
    const topLevelLabels = ["mssql", "http-dep-1", "kafka", "postgres", "parent-component"];
    for (const label of topLevelLabels) {
      const node = layout.nodes.find((n) => n.label === label);
      const hasOutgoingEdge = layout.edges.some((e) => e.source === node?.id);
      expect(hasOutgoingEdge, `top-level node "${label}" should have no outgoing edge`).toBe(false);
    }
  });

  it("dependency component_ids resolving to a top level component adds a decorative link", () => {
    const layout = computeLayout(
      arenaState([subject("postgres", "started", [], ["api"])], [subject("api", "started")])
    );

    const dep = layout.nodes.find((n) => n.label === "postgres");
    const comp = layout.nodes.find((n) => n.label === "api");
    const link = layout.edges.find((e) => e.decorative);

    expect(link).toBeDefined();
    expect(link?.source).toBe(dep?.id);
    expect(link?.target).toBe(comp?.id);
  });

  it("dependency component_ids referencing an unknown id adds no decorative edge", () => {
    const layout = computeLayout(arenaState([subject("postgres", "started", [], ["does-not-exist"])], []));

    expect(layout.edges.filter((e) => e.decorative)).toHaveLength(0);
  });

  it("dependency component_ids resolving to a nested component adds a decorative link", () => {
    const nested = subject("worker", "started");
    const layout = computeLayout(
      arenaState([subject("postgres", "started", [], ["worker"])], [subject("api", "started", [nested])])
    );

    const dep = layout.nodes.find((n) => n.label === "postgres");
    const comp = layout.nodes.find((n) => n.label === "worker");
    const link = layout.edges.find((e) => e.decorative);

    expect(link).toBeDefined();
    expect(link?.source).toBe(dep?.id);
    expect(link?.target).toBe(comp?.id);
  });

  it("not_started state is not visible and other states are visible", () => {
    const layout = computeLayout(
      arenaState([subject("postgres", "not_started"), subject("kafka", "started")], [])
    );

    expect(layout.nodes.find((n) => n.label === "postgres")?.visible).toBe(false);
    expect(layout.nodes.find((n) => n.label === "kafka")?.visible).toBe(true);
  });
});
