import { describe, expect, it } from "vitest";

import { computeLayout } from "./layout";
import type { ArenaState, Subject } from "./types";

function subject(id: string, state: string, children: Subject[] = [], dependency_ids?: string[]): Subject {
  return { id, state, faults: [], children, dependency_ids };
}

function arenaState(dependencies: Subject[], components: Subject[]): ArenaState {
  return { id: "arena-1", state: "arena_open", at: "2026-01-01T00:00:00.000Z", dependencies, components, faults: [] };
}

describe("computeLayout", () => {
  it("empty arena returns no nodes or edges", () => {
    const layout = computeLayout(arenaState([], []));

    expect(layout.nodes).toHaveLength(0);
    expect(layout.edges).toHaveLength(0);
  });

  it("top level subjects with no relationships produce nodes but no edges", () => {
    const layout = computeLayout(arenaState([subject("postgres", "started")], [subject("api", "started")]));

    expect(layout.nodes).toHaveLength(2);
    expect(layout.edges).toHaveLength(0);
    const dep = layout.nodes.find((n) => n.label === "postgres");
    expect(dep?.kind).toBe("dependency");
    expect(dep?.depth).toBe(0);
    const comp = layout.nodes.find((n) => n.label === "api");
    expect(comp?.kind).toBe("component");
  });

  it("nested child produces a structural edge from child to parent", () => {
    const child = subject("http-dep-2", "started");
    const parent = subject("postgres", "started", [child]);
    const layout = computeLayout(arenaState([parent], []));

    expect(layout.nodes).toHaveLength(2);
    const childNode = layout.nodes.find((n) => n.label === "http-dep-2");
    const parentNode = layout.nodes.find((n) => n.label === "postgres");
    expect(layout.edges).toHaveLength(1);
    expect(layout.edges[0].source).toBe(childNode?.id);
    expect(layout.edges[0].target).toBe(parentNode?.id);
    expect(layout.edges[0].decorative).toBe(false);
  });

  it("faulted child marks the structural edge as faulted", () => {
    const child = subject("http-dep-2", "faulted");
    const parent = subject("postgres", "started", [child]);
    const layout = computeLayout(arenaState([parent], []));

    expect(layout.edges[0].faulted).toBe(true);
  });

  it("component dependency_ids resolving to a top level dependency adds a decorative link", () => {
    const layout = computeLayout(
      arenaState([subject("postgres", "started")], [subject("api", "started", [], ["postgres"])])
    );

    const dep = layout.nodes.find((n) => n.label === "postgres");
    const comp = layout.nodes.find((n) => n.label === "api");
    const link = layout.edges.find((e) => e.decorative);

    expect(link).toBeDefined();
    expect(link?.source).toBe(dep?.id);
    expect(link?.target).toBe(comp?.id);
  });

  it("component dependency_ids referencing an unknown id adds no edge", () => {
    const layout = computeLayout(arenaState([], [subject("api", "started", [], ["does-not-exist"])]));

    expect(layout.edges).toHaveLength(0);
  });

  it("component dependency_ids cannot resolve a nested dependency directly", () => {
    const nested = subject("http-dep-2", "started");
    const layout = computeLayout(
      arenaState(
        [subject("postgres", "started", [nested])],
        [subject("api", "started", [], ["http-dep-2"])]
      )
    );

    const decorative = layout.edges.filter((e) => e.decorative);
    expect(decorative).toHaveLength(0);
  });

  it("not_started state is not visible and other states are visible", () => {
    const layout = computeLayout(
      arenaState([subject("postgres", "not_started"), subject("kafka", "started")], [])
    );

    expect(layout.nodes.find((n) => n.label === "postgres")?.visible).toBe(false);
    expect(layout.nodes.find((n) => n.label === "kafka")?.visible).toBe(true);
  });
});
